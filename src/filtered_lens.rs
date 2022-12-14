use druid::{ Data, Lens };
use druid::im::Vector;
use std::rc::Rc;

use fuzzy_matcher::skim::SkimMatcherV2;

pub struct FilteredLens { }

#[derive(Clone, Data, Debug)]
struct FilterMatch<T: Data>{
  text: T,
  index: usize,
  score: i64
}

pub trait FuzzyMatchable {
  fn match_against(&self) -> &str;
}

impl FuzzyMatchable for String {
  fn match_against(&self) -> &str {
    &self
  }
}

impl<T: FuzzyMatchable> FuzzyMatchable for Rc<T> {
  fn match_against(&self) -> &str {
    let s = self.as_ref();
    s.match_against()
  }
}

impl FilteredLens {
  fn match_items<L, F, V>(&self, list: &L, filter: &F) -> Vector<FilterMatch<V>>
where L: Data+IntoIterator<Item = V>,
    F: FuzzyMatchable,
    V: Data+FuzzyMatchable
  {
    let matcher = SkimMatcherV2::default();
    let f = filter.match_against();
    // XXX clone, collect
    list.clone().into_iter().enumerate().filter_map(|(index, text)| {
      let t = text.match_against();
      matcher.fuzzy(t, f, false).map(|(score,_)| FilterMatch{text: text.clone(), index, score})
    }).collect()
  }
}

impl<I, Q, L> Lens<(L,Q), L> for FilteredLens
where I: Data+FuzzyMatchable,
  Q: FuzzyMatchable,
  L: Data+IntoIterator<Item=I>+FromIterator<I>
{
  fn with<V, F: FnOnce(&L) -> V>(&self, (list, filter): &(L, Q), f: F) -> V {
    let matches = self.match_items(list, filter).iter().map(|fm| fm.text.clone()).collect();
    f(&matches)
  }

  fn with_mut<V, F: FnOnce(&mut L) -> V>(&self, (list, filter): &mut (L, Q), f: F) -> V {
    let mut matches = self.match_items(list, filter).iter().map(|fm| fm.text.clone()).collect();
    f(&mut matches)
  }
}
