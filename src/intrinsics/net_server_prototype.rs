use crate::interpreter::Context;
use crate::intrinsics::promise::new_promise_capability;
use crate::value::ObjectKey;
use crate::{Agent, Value};
use num::ToPrimitive;

fn next(agent: &Agent, _: Vec<Value>, ctx: &Context) -> Result<Value, Value> {
    let this = ctx.scope.borrow().get_this(agent)?;
    if !this.has_slot("net server queue") {
        return Err(Value::new_error(agent, "invalid receiver"));
    }

    if let Value::List(buffer) = this.get_slot("net server buffer") {
        if let Some(promise) = buffer.borrow_mut().pop_front() {
            return Ok(promise);
        }
    }

    if let Value::List(queue) = this.get_slot("net server queue") {
        let promise = new_promise_capability(agent, agent.intrinsics.promise.clone())?;
        queue.borrow_mut().push_back(promise.clone());
        Ok(promise)
    } else {
        unreachable!();
    }
}

fn close(agent: &Agent, _: Vec<Value>, ctx: &Context) -> Result<Value, Value> {
    let this = ctx.scope.borrow().get_this(agent)?;
    if !this.has_slot("net server token") {
        return Err(Value::new_error(agent, "invalid receiver"));
    }

    if let Value::Number(t) = this.get_slot("net server token") {
        let token = mio::Token(t.to_usize().unwrap());
        agent.mio_map.borrow_mut().remove(&token);
        Ok(Value::Null)
    } else {
        unreachable!();
    }
}

pub fn create_net_server_prototype(agent: &Agent) -> Value {
    let proto = Value::new_object(agent.intrinsics.async_iterator_prototype.clone());

    proto
        .set(
            agent,
            ObjectKey::from("next"),
            Value::new_builtin_function(agent, next),
        )
        .unwrap();

    proto
        .set(
            agent,
            ObjectKey::from("close"),
            Value::new_builtin_function(agent, close),
        )
        .unwrap();

    proto
}
