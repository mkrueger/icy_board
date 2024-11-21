use crate::{iemsi::{EMSI_IRQ_WITH_CLEAR, EMSI_NAK_WITH_CLEAR}, Connection};
use super::{decode_ici, EmsiICI, EmsiISI, EMSI_ACK};

pub async fn try_iemsi(
    com: &mut Box<dyn Connection>,
    name: String,
    location: String,
    operator: String,
    notice: String,
    capabilities: String,
) -> crate::Result<Option<EmsiICI>> {
    log::debug!("Trying IEMSI handshake");
    com.send(EMSI_IRQ_WITH_CLEAR).await?;
    let mut buf = [0; 1024];
    let iemsi_timeout = std::time::Instant::now();
    while iemsi_timeout.elapsed().as_millis() < 250 {
        let size = com.read(&mut buf).await?;
        if size == 0 {
            continue;
        }
        if let Ok(ici) = decode_ici(&buf[0..size]) {
            log::warn!("{}", String::from_utf8_lossy(&buf[0..size]));
            com.send(EMSI_ACK).await?;

            let isi = EmsiISI {
                id: format!("IcyBoard,{}", crate::VERSION.to_string()),
                name: name.into(),
                location: location.into(),
                operator: operator.into(),
                localtime: format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                notice: notice.into(),
                wait: "".to_string(),
                capabilities: capabilities.into(),
            };

            com.send(&isi.encode()?).await?;

            // In theory ACK/NAK should be sent here, but it's not really needed IMO.
            let instant = std::time::Instant::now();
            while instant.elapsed().as_millis() < 100 {
                let size = com.read(&mut buf).await?;
                if size == 0 {
                    continue;
                }
                break;
            }

            return Ok(Some(ici));
        } else {
            com.send(EMSI_NAK_WITH_CLEAR).await?;
            break;
        }
    }
    return Ok(None);

}
