use druid::{ Data, Lens };
use std::rc::Rc;

use fuzzy_matcher::skim::SkimMatcherV2;

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

#[derive(Default)]
pub struct FilteredLens {
  matcher: SkimMatcherV2
}

impl FilteredLens {
  fn match_items<'a, L, F, V>(&'a self, list: &L, filter: &'a F) -> impl Iterator<Item=FilterMatch<V>> + '_
where
    L: Data+IntoIterator<Item = V>,
    F: FuzzyMatchable,
    V: Data+FuzzyMatchable
  {
    let f = filter.match_against();
    // XXX clone, collect
    list.clone().into_iter().enumerate().filter_map(|(index, text)| {
      let t = text.match_against();
      self.matcher.fuzzy(t, f, false).map(|(score,_)| FilterMatch{text, index, score})
    })
  }
}

impl<L, M, Q, I> Lens<(L,Q), M> for FilteredLens
where I: Data+FuzzyMatchable,
  Q: FuzzyMatchable,
  L: Data+IntoIterator<Item=I>,
  M: Data+FromIterator<I>
{
  fn with<V, F: FnOnce(&M) -> V>(&self, (list, filter): &(L, Q), f: F) -> V {
    let matches = self.match_items(list, filter).map(|fm| fm.text).collect();
    f(&matches)
  }

  fn with_mut<V, F: FnOnce(&mut M) -> V>(&self, (list, filter): &mut (L, Q), f: F) -> V {
    let mut matches = self.match_items(list, filter).map(|fm| fm.text).collect();
    f(&mut matches)
  }
}
