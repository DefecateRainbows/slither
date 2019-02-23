use crate::agent::Agent;
use crate::value::{new_builtin_function, new_custom_object, new_error, ObjectKey, Value};
use crate::vm::ExecutionContext;

fn trigger_promise_reactions(
    agent: &Agent,
    reactions: Value,
    argument: Value,
) -> Result<Value, Value> {
    if let Value::List(list) = &reactions {
        loop {
            let item = list.borrow_mut().pop_front();
            match item {
                Some(reaction) => {
                    agent.enqueue_job(
                        promise_reaction_job,
                        vec![reaction.clone(), argument.clone()],
                    );
                }
                None => break,
            }
        }
    } else {
        unreachable!();
    }
    Ok(Value::Null)
}

pub fn promise_reaction_job(agent: &Agent, args: Vec<Value>) -> Result<(), Value> {
    let reaction = args[0].clone();
    let argument = args[1].clone();

    let promise = reaction.get_slot("promise");
    let kind = reaction.get_slot("kind");
    let handler = reaction.get_slot("handler");

    let mut handler_result;
    if handler == Value::Null {
        if kind == Value::from("fulfill") {
            handler_result = Ok(argument);
        } else {
            handler_result = Err(argument);
        }
    } else {
        handler_result = handler.call(agent, Value::Null, vec![argument]);
    }

    if promise != Value::Null {
        match handler_result {
            Ok(v) => promise
                .get_slot("resolve")
                .call(agent, Value::Null, vec![v])?,
            Err(v) => promise
                .get_slot("reject")
                .call(agent, Value::Null, vec![v])?,
        };
    }

    Ok(())
}

fn fulfill_promise(agent: &Agent, promise: Value, value: Value) -> Result<Value, Value> {
    let reactions = promise.get_slot("fulfill reactions");
    promise.set_slot("result", value.clone());
    promise.set_slot("promise state", Value::from("fulfilled"));
    promise.set_slot("fulfill reactions", Value::Null);
    promise.set_slot("reject reactions", Value::Null);
    trigger_promise_reactions(agent, reactions, value)
}

fn reject_promise(agent: &Agent, promise: Value, reason: Value) -> Result<Value, Value> {
    let reactions = promise.get_slot("reject reactions");
    promise.set_slot("result", reason.clone());
    promise.set_slot("promise state", Value::from("rejected"));
    promise.set_slot("fulfill reactions", Value::Null);
    promise.set_slot("reject reactions", Value::Null);
    trigger_promise_reactions(agent, reactions, reason)
}

struct ResolvingFunctions {
    resolve: Value,
    reject: Value,
}

fn create_resolving_functions(agent: &Agent, promise: &Value) -> ResolvingFunctions {
    let already_resolved = new_custom_object(Value::Null);
    already_resolved.set_slot("resolved", Value::False);

    let resolve = new_builtin_function(agent, promise_resolve_function);
    resolve.set_slot("promise", promise.clone());
    resolve.set_slot("already resolved", already_resolved.clone());

    let reject = new_builtin_function(agent, promise_reject_function);
    reject.set_slot("promise", promise.clone());
    reject.set_slot("already resolved", already_resolved);

    ResolvingFunctions { resolve, reject }
}

fn promise_resolve_function(
    agent: &Agent,
    ctx: &ExecutionContext,
    args: Vec<Value>,
) -> Result<Value, Value> {
    let f = ctx.function.clone().unwrap();

    let already_resolved = f.get_slot("already resolved");
    if already_resolved.get_slot("resolved") == Value::True {
        return Ok(Value::Null);
    } else {
        already_resolved.set_slot("resolved", Value::True);
    }

    let promise = f.get_slot("promise");
    let resolution = args.get(0).unwrap_or(&Value::Null).clone();
    if promise == resolution {
        reject_promise(
            agent,
            promise,
            new_error("cannot resolve a promise with itself"),
        )
    } else if resolution.has_slot("promise state") {
        let ResolvingFunctions { resolve, reject } = create_resolving_functions(agent, &promise);
        let then_call_result = resolution.get(&ObjectKey::from("then"))?.call(
            agent,
            resolution,
            vec![resolve, reject.clone()],
        );
        match then_call_result {
            Ok(v) => Ok(v),
            Err(e) => reject.call(agent, Value::Null, vec![e]),
        }
    } else {
        fulfill_promise(agent, promise, resolution)
    }
}

fn promise_reject_function(
    agent: &Agent,
    ctx: &ExecutionContext,
    args: Vec<Value>,
) -> Result<Value, Value> {
    let f = ctx.function.clone().unwrap();

    let already_resolved = f.get_slot("already resolved");
    if already_resolved.get_slot("resolved") == Value::True {
        return Ok(Value::Null);
    } else {
        already_resolved.set_slot("resolved", Value::True);
    }

    let promise = f.get_slot("promise");
    let resolution = args.get(0).unwrap_or(&Value::Null).clone();
    reject_promise(agent, promise, resolution)
}

fn promise(agent: &Agent, _ctx: &ExecutionContext, args: Vec<Value>) -> Result<Value, Value> {
    let executor = args[0].clone();

    if executor.type_of() != "function" {
        return Err(new_error("executor must be a function"));
    }

    let promise = new_custom_object(agent.intrinsics.promise_prototype.clone());
    promise.set_slot("promise state", Value::from("pending"));
    promise.set_slot("fulfill reactions", Value::new_list());
    promise.set_slot("reject reactions", Value::new_list());

    let ResolvingFunctions { resolve, reject } = create_resolving_functions(agent, &promise);

    let result = executor.call(agent, Value::Null, vec![resolve, reject.clone()]);

    if let Err(e) = result {
        reject.call(agent, Value::Null, vec![e])?;
    }

    Ok(promise)
}

fn get_capabilities_executor(
    _agent: &Agent,
    ctx: &ExecutionContext,
    args: Vec<Value>,
) -> Result<Value, Value> {
    let f = ctx.function.clone().unwrap();

    let resolve = args.get(0).unwrap_or(&Value::Null).clone();
    let reject = args.get(1).unwrap_or(&Value::Null).clone();

    if f.get_slot("resolve") != Value::Null || f.get_slot("reject") != Value::Null {
        return Err(new_error("type error"));
    }

    f.set_slot("resolve", resolve);
    f.set_slot("reject", reject);

    Ok(Value::Null)
}

pub fn new_promise_capability(agent: &Agent, constructor: Value) -> Result<Value, Value> {
    let executor = new_builtin_function(agent, get_capabilities_executor);
    executor.set_slot("resolve", Value::Null);
    executor.set_slot("reject", Value::Null);

    let promise = constructor.construct(agent, vec![executor.clone()])?;
    promise.set_slot("resolve", executor.get_slot("resolve"));
    promise.set_slot("reject", executor.get_slot("reject"));

    Ok(promise)
}

fn promise_resolve(
    agent: &Agent,
    ctx: &ExecutionContext,
    args: Vec<Value>,
) -> Result<Value, Value> {
    let x = args.get(0).unwrap_or(&Value::Null);
    let c = ctx.environment.borrow().this.clone().unwrap();
    if c.type_of() != "object" && c.type_of() != "function" {
        return Err(new_error("this must be an object"));
    }
    if x.has_slot("promise state") {
        let x_constructor = x.get(&ObjectKey::from("constructor"))?;
        if x_constructor == c {
            return Ok(x.clone());
        }
    }
    let capability = new_promise_capability(agent, c)?;
    capability
        .get_slot("resolve")
        .call(agent, Value::Null, vec![x.clone()])?;
    Ok(capability)
}

fn promise_reject(agent: &Agent, ctx: &ExecutionContext, args: Vec<Value>) -> Result<Value, Value> {
    let x = args.get(0).unwrap_or(&Value::Null);
    let c = ctx.environment.borrow().this.clone().unwrap();
    if c.type_of() != "object" && c.type_of() != "function" {
        return Err(new_error("this must be an object"));
    }
    let capability = new_promise_capability(agent, c)?;
    capability
        .get_slot("reject")
        .call(agent, Value::Null, vec![x.clone()])?;
    Ok(capability)
}

pub fn create_promise(agent: &Agent, prototype: Value) -> Value {
    let p = new_builtin_function(agent, promise);

    p.set(&ObjectKey::from("prototype"), prototype.clone())
        .unwrap();
    p.set(
        &ObjectKey::from("resolve"),
        new_builtin_function(agent, promise_resolve),
    )
    .unwrap();
    p.set(
        &ObjectKey::from("reject"),
        new_builtin_function(agent, promise_reject),
    )
    .unwrap();
    prototype
        .set(&ObjectKey::from("constructor"), p.clone())
        .unwrap();

    p
}
