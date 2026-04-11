use super::{EmsiICI, EmsiISI, decode_ici};
use crate::{
    Connection,
    iemsi::{EMSI_IRQ_WITH_CLEAR, EMSI_NAK_WITH_CLEAR},
};

const EMSI_ICI_PREFIX: &[u8; 10] = b"**EMSI_ICI";

fn find_ici_start(buf: &[u8]) -> Option<usize> {
    buf.windows(EMSI_ICI_PREFIX.len()).position(|window| window == EMSI_ICI_PREFIX)
}

fn parse_hex_digits(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }

    let mut result = 0usize;
    for &byte in bytes {
        let digit = match byte {
            b'0'..=b'9' => usize::from(byte - b'0'),
            b'a'..=b'f' => usize::from(byte - b'a') + 10,
            b'A'..=b'F' => usize::from(byte - b'A') + 10,
            _ => return None,
        };
        result = result.checked_mul(16)?.checked_add(digit)?;
    }
    Some(result)
}

fn get_complete_ici_packet_len(buf: &[u8]) -> Option<usize> {
    if !buf.starts_with(EMSI_ICI_PREFIX) || buf.len() < EMSI_ICI_PREFIX.len() + 4 {
        return None;
    }

    let payload_len = parse_hex_digits(&buf[EMSI_ICI_PREFIX.len()..EMSI_ICI_PREFIX.len() + 4])?;
    EMSI_ICI_PREFIX
        .len()
        .checked_add(4)?
        .checked_add(payload_len)?
        .checked_add(8)?
        .checked_add(1)
        .filter(|total_len| buf.len() >= *total_len)
}

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

        if let Some(start) = find_ici_start(&ici_buf) {
            if start > 0 {
                ici_buf.drain(..start);
            }

            if let Some(packet_len) = get_complete_ici_packet_len(&ici_buf) {
                match decode_ici(&ici_buf[..packet_len]) {
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
                        // Continue reading more data until timeout and respond with NAK.
                    }
                }
            }
        }
    }

    // Timeout or invalid data - send NAK
    if !ici_buf.is_empty() {
        com.send(EMSI_NAK_WITH_CLEAR).await?;
    }
    Ok(None)
}
