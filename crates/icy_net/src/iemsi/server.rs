use super::{EMSI_IIR, EmsiICI, EmsiISI, decode_ici};
use crate::{
    Connection,
    iemsi::{EMSI_IRQ_WITH_CLEAR, EMSI_NAK_WITH_CLEAR},
};

pub async fn try_iemsi(
    com: &mut Box<dyn Connection>,
    name: String,
    location: String,
    operator: String,
    notice: String,
    capabilities: String,
) -> crate::Result<Option<EmsiICI>> {
    com.send(EMSI_IRQ_WITH_CLEAR).await?;
    let mut buf = [0; 1024];
    let mut ici_buf = Vec::new();
    let iemsi_timeout = std::time::Instant::now();

    while iemsi_timeout.elapsed().as_millis() < 2000 {
        // Use try_read for non-blocking read to properly handle timeout
        let size = com.try_read(&mut buf).await?;
        if size == 0 {
            // Small delay to avoid busy-waiting
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }
        ici_buf.extend_from_slice(&buf[0..size]);

        // Client sends EMSI_IIR before EMSI_ICI - skip it if present
        let ici_data = if ici_buf.starts_with(EMSI_IIR) {
            &ici_buf[EMSI_IIR.len()..]
        } else {
            &ici_buf[..]
        };

        match decode_ici(ici_data) {
            Ok(ici) => {
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

                // Send ISI packet (server info) to client
                let isi_data = isi.encode()?;
                com.send(&isi_data).await?;

                // Wait for client to acknowledge with 2x EMSI_ACK
                let instant = std::time::Instant::now();
                while instant.elapsed().as_millis() < 500 {
                    let size = com.try_read(&mut buf).await?;
                    if size == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        continue;
                    }
                    break;
                }

                return Ok(Some(ici));
            }
            Err(_) => {
                // Continue reading more data
            }
        }
    }

    // Timeout or invalid data - send NAK
    if !ici_buf.is_empty() {
        com.send(EMSI_NAK_WITH_CLEAR).await?;
    }
    Ok(None)
}
