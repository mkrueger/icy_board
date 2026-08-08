//! Option negotiation after RFC 1143, the "Q Method".
//!
//! Answering every offer on the spot is what makes two naive peers talk past each
//! other forever: we agree to a WILL with a DO, the peer reads the DO as a fresh
//! request and offers again. Remembering where an option stands ends that, and it
//! is also the only way to tell an answer to our own request apart from a new one.
//!
//! The state is per option and per side. Both sides run the same machine, so
//! `Accept`/`Refuse` mean DO/DONT for the peer's side and WILL/WONT for ours.

/// The answer to send, in whichever direction the option is being negotiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    Accept,
    Refuse,
}

/// Where one option stands on one side. The `Opposite` variants are RFC 1143's
/// queue bit: a change we asked for while the previous one was still in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Negotiation {
    #[default]
    No,
    Yes,
    WantNo,
    WantNoOpposite,
    WantYes,
    WantYesOpposite,
}

impl Negotiation {
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Yes)
    }

    /// An incoming WILL or DO.
    pub fn on_positive(&mut self, agree: bool) -> Option<Reply> {
        match self {
            Self::No => {
                if agree {
                    *self = Self::Yes;
                    Some(Reply::Accept)
                } else {
                    Some(Reply::Refuse)
                }
            }
            Self::Yes => None,
            // The peer is answering something we already withdrew. RFC 1143 says to
            // take the state it implies and send nothing, or the two never settle.
            Self::WantNo => {
                *self = Self::No;
                None
            }
            Self::WantNoOpposite => {
                *self = Self::Yes;
                None
            }
            Self::WantYes => {
                *self = Self::Yes;
                None
            }
            Self::WantYesOpposite => {
                *self = Self::WantNo;
                Some(Reply::Refuse)
            }
        }
    }

    /// An incoming WONT or DONT.
    pub fn on_negative(&mut self) -> Option<Reply> {
        match self {
            Self::No => None,
            Self::Yes => {
                *self = Self::No;
                Some(Reply::Refuse)
            }
            Self::WantNo => {
                *self = Self::No;
                None
            }
            Self::WantNoOpposite => {
                *self = Self::WantYes;
                Some(Reply::Accept)
            }
            Self::WantYes | Self::WantYesOpposite => {
                *self = Self::No;
                None
            }
        }
    }

    /// We start the exchange ourselves.
    pub fn request(&mut self, enable: bool) -> Option<Reply> {
        if enable {
            match self {
                Self::No => {
                    *self = Self::WantYes;
                    Some(Reply::Accept)
                }
                Self::WantNo => {
                    *self = Self::WantNoOpposite;
                    None
                }
                Self::WantYesOpposite => {
                    *self = Self::WantYes;
                    None
                }
                Self::Yes | Self::WantNoOpposite | Self::WantYes => None,
            }
        } else {
            match self {
                Self::Yes => {
                    *self = Self::WantNo;
                    Some(Reply::Refuse)
                }
                Self::WantNoOpposite => {
                    *self = Self::WantNo;
                    None
                }
                Self::WantYes => {
                    *self = Self::WantYesOpposite;
                    None
                }
                Self::No | Self::WantNo | Self::WantYesOpposite => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offer_we_want_is_accepted_once() {
        let mut neg = Negotiation::default();
        assert_eq!(neg.on_positive(true), Some(Reply::Accept));
        assert!(neg.is_enabled());
        // This is the loop the old parser fell into: a peer that reads our answer as
        // a new request and offers again gets nothing back.
        assert_eq!(neg.on_positive(true), None);
        assert_eq!(neg.on_positive(true), None);
    }

    #[test]
    fn an_offer_we_do_not_want_is_refused_without_changing_state() {
        let mut neg = Negotiation::default();
        assert_eq!(neg.on_positive(false), Some(Reply::Refuse));
        assert_eq!(neg, Negotiation::No);
        assert_eq!(neg.on_positive(false), Some(Reply::Refuse));
    }

    #[test]
    fn a_refusal_of_something_that_was_never_on_is_not_answered() {
        let mut neg = Negotiation::default();
        assert_eq!(neg.on_negative(), None);
        assert_eq!(neg, Negotiation::No);
    }

    #[test]
    fn turning_an_agreed_option_off_is_confirmed_once() {
        let mut neg = Negotiation::default();
        neg.on_positive(true);
        assert_eq!(neg.on_negative(), Some(Reply::Refuse));
        assert!(!neg.is_enabled());
        assert_eq!(neg.on_negative(), None);
    }

    #[test]
    fn our_own_request_is_sent_once_and_the_answer_is_not_echoed() {
        let mut neg = Negotiation::default();
        assert_eq!(neg.request(true), Some(Reply::Accept));
        assert_eq!(neg, Negotiation::WantYes);
        // Asking again while the first ask is unanswered must not put a second
        // request on the wire.
        assert_eq!(neg.request(true), None);
        assert_eq!(neg.on_positive(true), None);
        assert!(neg.is_enabled());
    }

    #[test]
    fn a_request_the_peer_turns_down_leaves_the_option_off() {
        let mut neg = Negotiation::default();
        neg.request(true);
        assert_eq!(neg.on_negative(), None);
        assert_eq!(neg, Negotiation::No);
    }

    #[test]
    fn a_change_asked_for_while_one_is_in_flight_is_queued() {
        let mut neg = Negotiation::default();
        neg.on_positive(true);
        assert_eq!(neg.request(false), Some(Reply::Refuse));
        assert_eq!(neg.request(true), None);
        assert_eq!(neg, Negotiation::WantNoOpposite);
        // The peer confirms the disable, which releases the queued enable.
        assert_eq!(neg.on_negative(), Some(Reply::Accept));
        assert_eq!(neg, Negotiation::WantYes);
    }
}
