#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Member {
    pub position: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub name: String,
    pub year: String,
    pub members: Vec<Member>,
}
