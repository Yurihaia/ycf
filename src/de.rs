use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::forward_to_deserialize_any;

use crate::cursor::{Keyword, TokenKind};
use crate::error::{Error, Result};
use crate::parse::{ParseError, ParseErrorKind, Parser};

pub struct Deserializer<'de> {
    parser: Parser<'de>,
}

impl<'de> Deserializer<'de> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Self {
            parser: Parser::new(input),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parser.peek_token().token.kind {
            TokenKind::Keyword(Keyword::Null) => self.deserialize_unit(visitor),
            TokenKind::Keyword(Keyword::True | Keyword::False) => self.deserialize_bool(visitor),
            TokenKind::Integer { sign: false, .. } => self.deserialize_u64(visitor),
            TokenKind::Integer { sign: true, .. } => self.deserialize_i64(visitor),
            TokenKind::Float => self.deserialize_f64(visitor),
            TokenKind::String { .. } => self.deserialize_string(visitor),
            TokenKind::StartSquare => self.deserialize_seq(visitor),
            TokenKind::StartCurly => self.deserialize_map(visitor),
            _ => Err(Error::parse(
                self.parser.error(ParseErrorKind::UnknownToken),
            )),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parser.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parser.parse_int()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parser.parse_int()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parser.parse_int()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parser.parse_int()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parser.parse_uint()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parser.parse_uint()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parser.parse_uint()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parser.parse_uint()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parser.parse_float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parser.parse_float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.parser.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.parser.parse_string()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.parser.try_parse_null().is_some() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parser.parse_null()?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parser.start_list()?;
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parser.start_map()?;
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(r) = self.parser.try_parse_string() {
            visitor.visit_enum(r?.into_deserializer())
        } else if self.parser.try_start_map().is_some() {
            let v = visitor.visit_enum(&mut *self)?;
            self.parser.end_map()?;
            Ok(v)
        } else {
            Err(Error::custom("expected an enum"))
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, 'a> SeqAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.parser.try_end_list().is_some() {
            return Ok(None);
        }

        seed.deserialize(&mut **self).map(Some)
    }
}

impl<'de, 'a> MapAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.parser.try_end_map().is_some() {
            return Ok(None);
        }

        seed.deserialize(&mut KeyDeserializer { de: self })
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if self.parser.peek_no_skip().token.kind == TokenKind::Dot {
            self.parser.next_no_skip();
            seed.deserialize(&mut PathMapDeserializer {
                de: self,
                done: false,
            })
        } else {
            self.parser.map_delimiter()?;
            seed.deserialize(&mut **self)
        }
    }
}

struct KeyDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut KeyDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.de.parser.peek_no_skip();
        match next.token.kind {
            TokenKind::Ident => {
                self.de.parser.next_no_skip();
                visitor.visit_borrowed_str(self.de.parser.src(next.token))
            }
            _ => Err(Error::parse(ParseError::new(
                next,
                ParseErrorKind::ExpectedIdent,
            ))),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct PathMapDeserializer<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    done: bool,
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut PathMapDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(&mut *self.de)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

impl<'a, 'de> MapAccess<'de> for &'a mut PathMapDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.done {
            Ok(None)
        } else {
            seed.deserialize(&mut KeyDeserializer { de: self.de })
                .map(Some)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.done = true;
        if self.de.parser.peek_no_skip().token.kind == TokenKind::Dot {
            self.de.parser.next_no_skip();
            seed.deserialize(&mut PathMapDeserializer {
                de: self.de,
                done: false,
            })
        } else {
            self.de.parser.map_delimiter()?;
            seed.deserialize(&mut *self.de)
        }
    }
}

impl<'de, 'a> EnumAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut KeyDeserializer { de: self })
            .map(|v| (v, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.parser.map_delimiter()?;
        self.parser.parse_null()?;
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        self.parser.map_delimiter()?;
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parser.map_delimiter()?;
        de::Deserializer::deserialize_seq(self, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parser.map_delimiter()?;
        de::Deserializer::deserialize_map(self, visitor)
    }
}

pub struct TopDeserializer<'de> {
    de: Deserializer<'de>,
}

impl<'de> TopDeserializer<'de> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Self {
            de: Deserializer::from_str(input),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut TopDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de, 'a> MapAccess<'de> for &'a mut TopDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.parser.peek_eof() {
            Ok(None)
        } else {
            seed.deserialize(&mut KeyDeserializer { de: &mut self.de })
                .map(Some)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // println!("{:?}", self.de.parser.peek_token().token.kind);
        if self.de.parser.peek_no_skip().token.kind == TokenKind::Dot {
            self.de.parser.next_no_skip();
            seed.deserialize(&mut PathMapDeserializer {
                de: &mut self.de,
                done: false,
            })
        } else {
            self.de.parser.map_delimiter()?;
            seed.deserialize(&mut self.de)
        }
    }
}
