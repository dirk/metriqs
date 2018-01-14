use std::str::{self, FromStr};

use nom::{digit, is_alphanumeric, IResult};
use string_cache::DefaultAtom as Atom;

#[derive(Debug, PartialEq)]
pub struct ParseError {
    description: String,
}

#[derive(Debug, PartialEq)]
pub enum StatsdMetric {
    /// Name, value, sample rate
    Counter(Atom, f64, Option<f64>),
    /// Name, value
    Gauge(Atom, f64),
    /// Name, value, sample rate
    Timer(Atom, f64, Option<f64>),
}

pub fn parse_metrics<'a>(i: &'a [u8]) -> Result<Vec<StatsdMetric>, ParseError> {
    let result = complete!(i, call!(metrics));

    match result {
        IResult::Done(_, metrics) => Ok(metrics),
        IResult::Error(err) => Err(ParseError{ description: format!("{:?}", err) }),
        IResult::Incomplete(_) => unreachable!(),
    }
}

named!(metrics<Vec<StatsdMetric>>,
    separated_nonempty_list_complete!(
        tag!("\n"),
        alt_complete!(
            counter |
            gauge   |
            timer
        )
    )
);

named!(counter<StatsdMetric>,
    do_parse!(
                name: metric_name                  >>
                      tag!(":")                    >>
               value: double                       >>
                      tag!("|c")                   >>
        _sample_rate: opt!(complete!(sample_rate)) >>

        (StatsdMetric::Counter(Atom::from(name), value, None))
    )
);

named!(gauge<StatsdMetric>,
    do_parse!(
         name: metric_name >>
               tag!(":")   >>
        value: double      >>
               tag!("|g")  >>

        (StatsdMetric::Gauge(Atom::from(name), value))
    )
);

named!(timer<StatsdMetric>,
    do_parse!(
         name: metric_name >>
               tag!(":")   >>
        value: double      >>
               tag!("|ms") >>

        (StatsdMetric::Timer(Atom::from(name), value, None))
    )
);

named!(metric_name<&str>,
    map_res!(
        take_while!(call!(|c| {
            is_alphanumeric(c) || c == b'.' || c == b'_'
        })),
        str::from_utf8
    )
);

named!(sample_rate<&[u8], f64>,
    preceded!(
        tag!("|@"),
        double
    )
);

named!(double<&[u8], f64>,
    map_res!(
        map_res!(
            recognize!(
                tuple!(
                    opt!(tag!("-")),
                    alt_complete!(
                        delimited!(digit, tag!("."), opt!(complete!(digit))) |
                        delimited!(opt!(digit), tag!("."), digit)            |
                        digit
                    )
                )
            ),
            str::from_utf8
        ),
        f64::from_str
    )
);

#[cfg(test)]
mod tests {
    use super::*;

    use std::any::Any;
    use nom::IResult;

    fn complete<'a, T>(value: T) -> IResult<&'a [u8], T>
        where T: Any {
        IResult::Done(&b""[..], value)
    }

    #[test]
    fn it_parses_metric_names() {
        assert_eq!(
            metric_name(&b"foo"[..]),
            complete("foo")
        );
        assert_eq!(
            metric_name(&b"foo.bar"[..]),
            complete("foo.bar")
        );
        assert_eq!(
            metric_name(&b"foo_bar"[..]),
            complete("foo_bar")
        );
    }

    #[test]
    fn it_parses_doubles() {
        // Integer
        assert_eq!(
            double(&b"23"[..]),
            complete(23.0)
        );
        // Fractional
        assert_eq!(
            double(&b".24"[..]),
            complete(0.24)
        );
        // Integer and fractional
        assert_eq!(
            double(&b"2.5"[..]),
            complete(2.5)
        );
        // Negative
        assert_eq!(
            double(&b"-2"[..]),
            complete(-2.0)
        );
    }

    #[test]
    fn it_parses_counter() {
        assert_eq!(
            counter(&b"foo.bar_baz:23|c"[..]),
            complete(StatsdMetric::Counter(Atom::from("foo.bar_baz"), 23.0, None))
        );
    }

    #[test]
    fn it_parses_gauge() {
        assert_eq!(
            gauge(&b"foo.bar_baz:12|g"[..]),
            complete(StatsdMetric::Gauge(Atom::from("foo.bar_baz"), 12.0))
        );
    }

    #[test]
    fn it_parses_timer() {
        assert_eq!(
            timer(&b"foo.bar_baz:12|ms"[..]),
            complete(StatsdMetric::Timer(Atom::from("foo.bar_baz"), 12.0, None))
        );
    }

    #[test]
    fn it_parses_metrics() {
        assert_eq!(
            parse_metrics(&b"foo:1|g\nbar:2|c|@3\nbaz:4|ms"[..]),
            Ok(vec![
                StatsdMetric::Gauge(Atom::from("foo"), 1.0),
                StatsdMetric::Counter(Atom::from("bar"), 2.0, None),
                StatsdMetric::Timer(Atom::from("baz"), 4.0, None),
            ])
        );
    }
}
