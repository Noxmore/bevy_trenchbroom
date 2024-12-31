//! TODO remove

use crate::*;

pub(crate) fn parse_qmap(bytes: &[u8]) -> io::Result<quake_util::qmap::QuakeMap> {
    quake_util::qmap::parse(&mut io::BufReader::new(bytes))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}