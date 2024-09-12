use crate::notification::Notification;
use waglayla_notify::{connection::ChannelConnection, notifier::Notifier};

pub type IndexNotifier = Notifier<Notification, ChannelConnection<Notification>>;
