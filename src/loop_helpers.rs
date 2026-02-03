//! Iterator helpers.
//!
//! Port of Python Rich's `rich/_loop.py`.

/// Iterate and generate a tuple with a flag for first value.
pub fn loop_first<I>(values: I) -> LoopFirst<I::IntoIter>
where
    I: IntoIterator,
{
    LoopFirst {
        iter: values.into_iter(),
        is_first: true,
    }
}

/// Iterate and generate a tuple with a flag for last value.
pub fn loop_last<I>(values: I) -> LoopLast<I::IntoIter>
where
    I: IntoIterator,
{
    LoopLast {
        iter: values.into_iter(),
        previous: None,
        done: false,
    }
}

/// Iterate and generate a tuple with a flag for first and last value.
pub fn loop_first_last<I>(values: I) -> LoopFirstLast<I::IntoIter>
where
    I: IntoIterator,
{
    LoopFirstLast {
        iter: values.into_iter(),
        previous: None,
        is_first: true,
        done: false,
    }
}

pub struct LoopFirst<I> {
    iter: I,
    is_first: bool,
}

impl<I> Iterator for LoopFirst<I>
where
    I: Iterator,
{
    type Item = (bool, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        let first = self.is_first;
        self.is_first = false;
        Some((first, item))
    }
}

pub struct LoopLast<I>
where
    I: Iterator,
{
    iter: I,
    previous: Option<I::Item>,
    done: bool,
}

impl<I> Iterator for LoopLast<I>
where
    I: Iterator,
{
    type Item = (bool, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.previous.is_none() {
            self.previous = self.iter.next();
            if self.previous.is_none() {
                self.done = true;
                return None;
            }
        }

        let next_value = self.iter.next();
        match next_value {
            Some(value) => {
                let previous = self.previous.replace(value).expect("previous initialized");
                Some((false, previous))
            }
            None => {
                self.done = true;
                let previous = self.previous.take().expect("previous initialized");
                Some((true, previous))
            }
        }
    }
}

pub struct LoopFirstLast<I>
where
    I: Iterator,
{
    iter: I,
    previous: Option<I::Item>,
    is_first: bool,
    done: bool,
}

impl<I> Iterator for LoopFirstLast<I>
where
    I: Iterator,
{
    type Item = (bool, bool, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.previous.is_none() {
            self.previous = self.iter.next();
            if self.previous.is_none() {
                self.done = true;
                return None;
            }
        }

        let next_value = self.iter.next();
        match next_value {
            Some(value) => {
                let previous = self.previous.replace(value).expect("previous initialized");
                let first = self.is_first;
                self.is_first = false;
                Some((first, false, previous))
            }
            None => {
                self.done = true;
                let previous = self.previous.take().expect("previous initialized");
                let first = self.is_first;
                self.is_first = false;
                Some((first, true, previous))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_first_empty() {
        let items: Vec<(bool, i32)> = loop_first(std::iter::empty::<i32>()).collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_loop_first_singleton() {
        let items: Vec<(bool, i32)> = loop_first([1]).collect();
        assert_eq!(items, vec![(true, 1)]);
    }

    #[test]
    fn test_loop_first_multi() {
        let items: Vec<(bool, i32)> = loop_first([1, 2, 3]).collect();
        assert_eq!(items, vec![(true, 1), (false, 2), (false, 3)]);
    }

    #[test]
    fn test_loop_last_empty() {
        let items: Vec<(bool, i32)> = loop_last(std::iter::empty::<i32>()).collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_loop_last_singleton() {
        let items: Vec<(bool, i32)> = loop_last([1]).collect();
        assert_eq!(items, vec![(true, 1)]);
    }

    #[test]
    fn test_loop_last_multi() {
        let items: Vec<(bool, i32)> = loop_last([1, 2, 3]).collect();
        assert_eq!(items, vec![(false, 1), (false, 2), (true, 3)]);
    }

    #[test]
    fn test_loop_first_last_empty() {
        let items: Vec<(bool, bool, i32)> = loop_first_last(std::iter::empty::<i32>()).collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_loop_first_last_singleton() {
        let items: Vec<(bool, bool, i32)> = loop_first_last([1]).collect();
        assert_eq!(items, vec![(true, true, 1)]);
    }

    #[test]
    fn test_loop_first_last_multi() {
        let items: Vec<(bool, bool, i32)> = loop_first_last([1, 2, 3]).collect();
        assert_eq!(
            items,
            vec![(true, false, 1), (false, false, 2), (false, true, 3)]
        );
    }
}
