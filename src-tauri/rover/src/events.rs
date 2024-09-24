


pub enum Event
{
    TaskStatus,
    TaskEnd,
}

// TODO I imagine we could also describe the payloads for each event kind here, with functions to help provide them...

impl Event
{
    pub fn event_name(&self) -> &str
    {
        match self
        {
            Event::TaskStatus => "task-status",
            Event::TaskEnd => "task-end",
        }
    }
}