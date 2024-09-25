use serde::{Deserialize, Serialize};




pub enum Event
{
    TaskStatus,
    TaskEnd,
}

// TODO I imagine we could also describe the payloads for each event kind here, with functions to help provide them...
//    At least part of that would be a task UUID, I suppose, to match up TaskStatus and TaskEnd.
// We'd want to match up the payloads here (move from the payload module)
// and the payload in Payload.tsx.
//   We can obviously have multiple payload types, probably one per event kind.

impl Event
{
    pub fn event_name(&self) -> &str
    {
        match self
        {
            // TODO These strings need matching handlers on the frontend. Our hashmap idea, I think?
            Event::TaskStatus => "task-status",
            Event::TaskEnd => "task-end",
        }
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct TaskStatusPayload
{
    pub task_uuid: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct TaskEndPayload
{
    pub task_uuid: String,
}