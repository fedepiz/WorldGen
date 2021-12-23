use std::{marker::PhantomData};

pub trait FinIdx: Clone + Copy + std::hash::Hash + PartialEq + Eq + PartialOrd + Ord + From<usize> + Into<usize> {}

#[macro_export]
macro_rules! fin_idx {
    ($vis:vis $name:ident) => {

        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, std::hash::Hash)]
        $vis struct $name(usize);
    
        impl From<usize> for $name {
            fn from(u:usize) -> Self {
                Self(u)
            }
        }

        impl From<$name> for usize {
            fn from(s:$name) -> usize { s.0 }
        }

        impl finvec::FinIdx for $name {}
    };
}


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FinDef<K, V> {
    key: PhantomData<K>,
    data: Vec<V>,
}

impl <K:FinIdx, V> FinDef<K, V> {
    pub fn new(data: Vec<V>) -> Self {
        Self {
            key: PhantomData,
            data
        }
    }

    pub fn idx_from_level(&self, tgt: f64, level: impl Fn(&V) -> f64) -> usize {
        self.data
            .iter()
            .enumerate()
            .find_map(|(idx, v)| if tgt < level(v) { Some(idx) } else { None })
            .unwrap()
    }

    pub fn from_level(&self, tgt: f64,level: impl Fn(&V) -> f64) -> K {
        K::from(self.idx_from_level(tgt, level))
    }

    pub fn from_level_range(&self, tgt: f64, level: impl Fn(&V) -> f64 + Copy) -> (K, K, f64) {
        let high_idx = self.idx_from_level(tgt, level);

        let low_idx = high_idx.saturating_sub(1);
        
        let high_l = level(&self.data[high_idx]);
        let low_l =  level(&self.data[low_idx]);

        let n = high_l - low_l;
        let t = if n == 0.0 {
            1.0
        } else {
            (tgt - low_l) / (high_l - low_l)
        };
        (K::from(low_idx), K::from(high_idx), t)
    }
}

impl <K:FinIdx, V> std::ops::Index<K> for FinDef<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.into()]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FinVec<K, V> {
    key: PhantomData<K>,
    data: Vec<V>,
}

impl <K:FinIdx, V:Copy> FinVec<K, V> {
    pub fn fill<T>(def: &FinDef<K, T>, default: V) -> Self {
        Self::tabulate(def, |_| default)
    }

    pub fn tabulate<T>(def: &FinDef<K, T>, mut f: impl FnMut(K) -> V) -> Self  {
        let data = (0 .. def.data.len()).map(|k| f(k.into())).collect();

        Self {
            key: PhantomData,
            data,
        }
    }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn iter(&self) -> impl Iterator<Item=(K,&V)> + '_ {
        self.data.iter().enumerate().map(|(idx, v)| (K::from(idx), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=(K,&mut V)> + '_ {
        self.data.iter_mut().enumerate().map(|(idx, v)| (K::from(idx), v))
    }
}

impl <K:FinIdx, V:Copy> std::ops::Index<K> for FinVec<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.into()]
    }
}


impl <K:FinIdx, V:Copy> std::ops::IndexMut<K> for FinVec<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.into()]
    }
}



