use serde::de;
use Value;
use super::{Decoder, DecodeError, DecodeErrorKind};

struct DecodeValue(Value);
struct MapVisitor<I>(I, Option<Value>);
struct SubDecoder(Decoder);

fn se2toml(err: de::value::Error, ty: &'static str) -> DecodeError {
    match err {
        de::value::Error::SyntaxError => de::Error::syntax_error(),
        de::value::Error::EndOfStreamError => de::Error::end_of_stream_error(),
        de::value::Error::MissingFieldError(s) => {
            DecodeError {
                field: Some(s.to_string()),
                kind: DecodeErrorKind::ExpectedField(Some(ty)),
            }
        }
    }
}

impl de::Deserializer for Decoder {
    type Error = DecodeError;

    fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, DecodeError>
        where V: de::Visitor
    {
        match self.toml.take() {
            Some(Value::String(s)) => {
                visitor.visit_string(s).map_err(|e| se2toml(e, "string"))
            }
            Some(Value::Integer(i)) => {
                visitor.visit_i64(i).map_err(|e| se2toml(e, "integer"))
            }
            Some(Value::Float(f)) => {
                visitor.visit_f64(f).map_err(|e| se2toml(e, "float"))
            }
            Some(Value::Boolean(b)) => {
                visitor.visit_bool(b).map_err(|e| se2toml(e, "bool"))
            }
            Some(Value::Datetime(s)) => {
                visitor.visit_string(s).map_err(|e| se2toml(e, "date"))
            }
            Some(Value::Array(a)) => {
                let len = a.len();
                let iter = a.into_iter().map(DecodeValue);
                let e = visitor.visit_seq(de::value::SeqDeserializer::new(iter,
                                                                          len));
                e.map_err(|e| se2toml(e, "array"))
            }
            Some(Value::Table(t)) => {
                visitor.visit_map(MapVisitor(t.into_iter(), None))
            }
            None => Err(de::Error::end_of_stream_error()),
        }
    }

    fn visit_option<V>(&mut self, mut visitor: V) -> Result<V::Value, DecodeError>
        where V: de::Visitor
    {
        if self.toml.is_none() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<V::Value, DecodeError>
        where V: de::Visitor,
    {
        if self.toml.is_none() {
            let iter = None::<i32>.into_iter();
            let e = visitor.visit_seq(de::value::SeqDeserializer::new(iter, 0));
            e.map_err(|e| se2toml(e, "array"))
        } else {
            self.visit(visitor)
        }
    }
}

impl de::Error for DecodeError {
    fn syntax_error() -> DecodeError {
        DecodeError { field: None, kind: DecodeErrorKind::SyntaxError }
    }
    fn end_of_stream_error() -> DecodeError {
        DecodeError { field: None, kind: DecodeErrorKind::EndOfStream }
    }
    fn missing_field_error(name: &'static str) -> DecodeError {
        DecodeError {
            field: Some(name.to_string()),
            kind: DecodeErrorKind::ExpectedField(None),
        }
    }
}

impl de::Deserializer for SubDecoder {
    type Error = de::value::Error;

    fn visit<V>(&mut self, visitor: V) -> Result<V::Value, de::value::Error>
        where V: de::Visitor
    {
        self.0.visit(visitor).map_err(|e| {
            match e.kind {
                DecodeErrorKind::SyntaxError => de::value::Error::SyntaxError,
                DecodeErrorKind::EndOfStream => de::value::Error::EndOfStreamError,
                _ => de::value::Error::SyntaxError,
            }
        })
    }
}

impl de::value::ValueDeserializer for DecodeValue {
    type Deserializer = SubDecoder;

    fn into_deserializer(self) -> SubDecoder {
        SubDecoder(Decoder::new(self.0))
    }
}

impl<I> de::MapVisitor for MapVisitor<I>
    where I: Iterator<Item=(String, Value)>
{
    type Error = DecodeError;

    fn visit_key<K>(&mut self) -> Result<Option<K>, DecodeError>
        where K: de::Deserialize
    {
        match self.0.next() {
            Some((k, v)) => {
                self.1 = Some(v);
                de::Deserialize::deserialize(&mut Decoder::new(Value::String(k)))
                   .map(|v| Some(v))
            }
            None => Ok(None),
        }

    }

    fn visit_value<V>(&mut self) -> Result<V, DecodeError>
        where V: de::Deserialize
    {
        match self.1.take() {
            Some(t) => de::Deserialize::deserialize(&mut Decoder::new(t)),
            None => Err(de::Error::end_of_stream_error())
        }
    }

    fn end(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }

}
