#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Either<L, R> {
  Left(L),
  Right(R),
}

impl<Item, L, R> Iterator for Either<L, R>
where
  L: Iterator<Item = Item>,
  R: Iterator<Item = Item>,
{
  type Item = Item;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      Either::Left(iter) => iter.next(),
      Either::Right(iter) => iter.next(),
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    match self {
      Either::Left(iter) => iter.size_hint(),
      Either::Right(iter) => iter.size_hint(),
    }
  }
}
