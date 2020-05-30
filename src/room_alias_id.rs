//! Matrix room alias identifiers.

use std::{borrow::Cow, convert::TryFrom, num::NonZeroU8};

#[cfg(feature = "diesel")]
use diesel::sql_types::Text;

use crate::{error::Error, parse_id};

/// A Matrix room alias ID.
///
/// A `RoomAliasId` is converted from a string slice, and can be converted back into a string as
/// needed.
///
/// ```
/// # use std::convert::TryFrom;
/// # use ruma_identifiers::RoomAliasId;
/// assert_eq!(
///     RoomAliasId::try_from("#ruma:example.com").unwrap().as_ref(),
///     "#ruma:example.com"
/// );
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "diesel", derive(FromSqlRow, QueryId, AsExpression, SqlType))]
#[cfg_attr(feature = "diesel", sql_type = "Text")]
pub struct RoomAliasId {
    pub(crate) full_id: Box<str>,
    pub(crate) colon_idx: NonZeroU8,
}

impl RoomAliasId {
    /// Returns the room's alias.
    pub fn alias(&self) -> &str {
        &self.full_id[1..self.colon_idx.get() as usize]
    }

    /// Returns the server name of the room alias ID.
    pub fn server_name(&self) -> &str {
        &self.full_id[self.colon_idx.get() as usize + 1..]
    }
}

impl TryFrom<Cow<'_, str>> for RoomAliasId {
    type Error = Error;

    /// Attempts to create a new Matrix room alias ID from a string representation.
    ///
    /// The string must include the leading # sigil, the alias, a literal colon, and a server name.
    fn try_from(room_id: Cow<'_, str>) -> Result<Self, Error> {
        let colon_idx = parse_id(&room_id, &['#'])?;

        Ok(Self {
            full_id: room_id.into_owned().into(),
            colon_idx,
        })
    }
}

common_impls!(RoomAliasId, "a Matrix room alias ID");

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    #[cfg(feature = "serde")]
    use serde_json::{from_str, to_string};

    use super::RoomAliasId;
    use crate::error::Error;

    #[test]
    fn valid_room_alias_id() {
        assert_eq!(
            RoomAliasId::try_from("#ruma:example.com")
                .expect("Failed to create RoomAliasId.")
                .as_ref(),
            "#ruma:example.com"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_valid_room_alias_id() {
        assert_eq!(
            to_string(
                &RoomAliasId::try_from("#ruma:example.com").expect("Failed to create RoomAliasId.")
            )
            .expect("Failed to convert RoomAliasId to JSON."),
            r##""#ruma:example.com""##
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_valid_room_alias_id() {
        assert_eq!(
            from_str::<RoomAliasId>(r##""#ruma:example.com""##)
                .expect("Failed to convert JSON to RoomAliasId"),
            RoomAliasId::try_from("#ruma:example.com").expect("Failed to create RoomAliasId.")
        );
    }

    #[test]
    fn valid_room_alias_id_with_explicit_standard_port() {
        assert_eq!(
            RoomAliasId::try_from("#ruma:example.com:443")
                .expect("Failed to create RoomAliasId.")
                .as_ref(),
            "#ruma:example.com:443"
        );
    }

    #[test]
    fn valid_room_alias_id_with_non_standard_port() {
        assert_eq!(
            RoomAliasId::try_from("#ruma:example.com:5000")
                .expect("Failed to create RoomAliasId.")
                .as_ref(),
            "#ruma:example.com:5000"
        );
    }

    #[test]
    fn valid_room_alias_id_unicode() {
        assert_eq!(
            RoomAliasId::try_from("#老虎Â£я:example.com")
                .expect("Failed to create RoomAliasId.")
                .as_ref(),
            "#老虎Â£я:example.com"
        );
    }

    #[test]
    fn missing_room_alias_id_sigil() {
        assert_eq!(
            RoomAliasId::try_from("39hvsi03hlne:example.com").unwrap_err(),
            Error::MissingSigil
        );
    }

    #[test]
    fn missing_localpart() {
        assert_eq!(
            RoomAliasId::try_from("#:example.com").unwrap_err(),
            Error::InvalidLocalPart
        );
    }

    #[test]
    fn missing_room_alias_id_delimiter() {
        assert_eq!(
            RoomAliasId::try_from("#ruma").unwrap_err(),
            Error::MissingDelimiter
        );
    }

    #[test]
    fn invalid_room_alias_id_host() {
        assert_eq!(
            RoomAliasId::try_from("#ruma:/").unwrap_err(),
            Error::InvalidServerName
        );
    }

    #[test]
    fn invalid_room_alias_id_port() {
        assert_eq!(
            RoomAliasId::try_from("#ruma:example.com:notaport").unwrap_err(),
            Error::InvalidServerName
        );
    }
}
