pub enum CommandPath {
    Root(&'static str),
    Sub(Vec<&'static str>),
}

impl From<Vec<&'static str>> for CommandPath {
    fn from(x: Vec<&'static str>) -> Self {
        CommandPath::Sub(x)
    }
}

impl From<&'static str> for CommandPath {
    fn from(x: &'static str) -> Self {
        CommandPath::Root(x)
    }
}
