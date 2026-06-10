#[derive(Debug, Default, Clone)]
pub struct ObsNamedList {
    pub items: Vec<String>,
    pub current: Option<String>,
}
