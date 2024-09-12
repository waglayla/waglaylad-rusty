use crate::notification::Notification;
use waglayla_notify::{connection::ChannelConnection, notifier::Notifier};

pub type ConsensusNotifier = Notifier<Notification, ChannelConnection<Notification>>;
