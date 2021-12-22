use std::collections::HashMap;


pub trait Space:Sized {
    type Tag: Clone + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash;

    fn make_params() -> Parameters<Self> {
        Parameters::new()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamId(usize);

pub struct Info<T:Space> {
    pub tag: T::Tag,
    pub name: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub logarithmic: bool,
}

pub struct Parameters<T:Space> {
    info: Vec<Info<T>>,
    values: Vec<f64>,
    tags: HashMap<T::Tag, ParamId>
}

impl <T:Space> Parameters<T> {
    pub fn new() -> Self {
        Self {
            info: vec![],
            values: vec![],
            tags: HashMap::new(),
        }
    }

    pub fn define(&mut self, info: Info<T>, value: f64) -> ParamId {
        assert!(!self.tags.contains_key(&info.tag));
        let id = ParamId(self.num_params());
        self.tags.insert(info.tag.clone(), id);
        self.info.push(info);
        self.values.push(value);
        id
    }

    pub fn info(&self, id: ParamId) -> &Info<T> {
        &self.info[id.0]
    }

    pub fn set_param(&mut self, p: ParamId, v: f64) {
        self.values[p.0] = v;
    }

    pub fn num_params(&self) -> usize { self.info.len() }

    pub fn get(&self, tag: &T::Tag) -> f64 {
        let id = self.tags[tag];
        self.values[id.0]
    }

    pub fn lookup(&self, tag: &T::Tag) -> ParamId { self.tags[tag] }
}

impl <T:Space> std::ops::Index<ParamId> for Parameters<T> {
    type Output = f64;
    
    fn index(&self, index: ParamId) -> &Self::Output {
        &self.values[index.0]
    }
}