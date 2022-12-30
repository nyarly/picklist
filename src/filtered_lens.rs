use druid::{ Data, Lens,FontWeight };
use druid::text::{Attribute,RichText};
use std::rc::Rc;
use std::sync::Arc;
use std::ops::RangeInclusive;

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

pub struct FilteredLens {
  matcher: SkimMatcherV2,
  markup: fn(&mut RichText, RangeInclusive<usize>)
}

impl Default for FilteredLens {
  fn default() -> Self {
    FilteredLens{
      matcher: SkimMatcherV2::default(),
      markup: |rt, bounds| {
        rt.add_attribute(bounds.clone(), Attribute::underline(true));
        rt.add_attribute(bounds.clone(), Attribute::weight(FontWeight::BOLD))
      }
    }
  }
}

impl FilteredLens {
  fn matched_pairs<'a, L, F, V>(&'a self, list: &L, filter: &'a F) -> impl Iterator<Item=(V, RichText)> + '_
where
    L: Data+IntoIterator<Item = V>,
    F: FuzzyMatchable,
    V: Data+FuzzyMatchable
  {
    let markup = self.markup;
    let f = filter.match_against();
    // XXX clone
    list.clone().into_iter().filter_map(move |text| {
      let t = text.match_against();
      self.matcher.fuzzy(t, f, true).map(|(_,pos)| {
        let mut rt = RichText::new(Arc::from(t.clone()));
        for range in runs_to_ranges(pos) {
          markup(&mut rt, range)
        }

        (text.clone(), rt)
      })
    })
  }

}

fn runs_to_ranges(indices: Vec<usize>) -> Vec<RangeInclusive<usize>> {
  if indices.len() == 0 {
    return vec![]
  }
  let mut left = indices[0];
  let mut right = indices[0];
  let mut ranges = vec![];
  for pair in indices.windows(2) {
    let l = pair[0]; let r = pair[1];
    if r - l > 1 {
      ranges.push(left..=right);
      left = r;
      right = r
    } else {
      right = r
    }
  }
  ranges.push(left..=right);
  ranges
}

#[cfg(test)]
mod tests {
  use super::runs_to_ranges;

  #[test]
  fn test_runs_to_ranges() {
    assert_eq!(runs_to_ranges(vec![]), vec![]);
    assert_eq!(runs_to_ranges(vec![0,1,2,3,4]), vec![0..=4]);
    assert_eq!(runs_to_ranges(vec![0,1,3,4]), vec![0..=1, 3..=4]);
    assert_eq!(runs_to_ranges(vec![0,2,4]), vec![0..=0, 2..=2, 4..=4]);
  }

}

impl<L, M, Q, I> Lens<(L,Q), M> for FilteredLens
where I: Data+FuzzyMatchable,
  Q: FuzzyMatchable,
  L: Data+IntoIterator<Item=I>,
  M: Data+FromIterator<(I, RichText)>
{
  fn with<V, F: FnOnce(&M) -> V>(&self, (list, filter): &(L, Q), f: F) -> V {
    let matches = self.matched_pairs(list, filter).collect();
    f(&matches)
  }

  fn with_mut<V, F: FnOnce(&mut M) -> V>(&self, (list, filter): &mut (L, Q), f: F) -> V {
    let mut matches = self.matched_pairs(list, filter).collect();
    f(&mut matches)
  }
}
