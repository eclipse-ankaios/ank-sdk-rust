// Copyright (c) 2025 Elektrobit Automotive GmbH
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains structs and enums that are used to
//! handle events in the Ankaios SDK.
//!
//! # Example
//!
//! ## Get information out of an `EventEntry`:
//!
//! ```rust
//! # use tokio::sync::mpsc;
//! # use ankaios_sdk::EventEntry;
//! #
//! let event_entry: EventEntry;
//! # let event_entry = EventEntry::default();
//! println!("Current complete state: {:?}", event_entry.complete_state);
//! for field in event_entry.added_fields {
//!     println!("Added field: {}", field);
//! }
//! for field in event_entry.updated_fields {
//!     println!("Updated field: {}", field);
//! }
//! for field in event_entry.removed_fields {
//!     println!("Removed field: {}", field);
//! }
//! ```
//!
//! ## Listen for events in an events campaign response:
//!
//! ```rust,no_run
//! # use ankaios_sdk::EventsCampaignResponse;
//! # use tokio::{sync::mpsc, runtime::Runtime};
//! use ankaios_sdk::EventEntry;
//! #
//! # Runtime::new().unwrap().block_on(async {
//! #
//! let events_campaign: EventsCampaignResponse;
//! # let (_events_sender, events_receiver) = mpsc::channel(1);
//! # let mut events_campaign = EventsCampaignResponse::new(String::default(), events_receiver);
//! while let Some(event_entry) = events_campaign.events_receiver.recv().await {
//! }
//! # })
//! ```

use tokio::sync::mpsc::Receiver;

use crate::{CompleteState, ankaios_api::ank_base::CompleteStateResponse};

/// Struct that represents an event notification.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EventEntry {
    /// The complete state of the event containing the changed state data.
    pub complete_state: CompleteState,
    /// The list of added fields of the state.
    pub added_fields: Vec<String>,
    /// The list of updated fields of the state.
    pub updated_fields: Vec<String>,
    /// The list of removed fields of the state.
    pub removed_fields: Vec<String>,
}

impl From<CompleteStateResponse> for EventEntry {
    fn from(value: CompleteStateResponse) -> Self {
        let altered_fields = value.altered_fields.unwrap_or_default();
        EventEntry {
            complete_state: CompleteState::new_from_proto(
                value
                    .complete_state
                    .expect("Complete State response must contain Complete State."),
            ),
            added_fields: altered_fields.added_fields,
            updated_fields: altered_fields.updated_fields,
            removed_fields: altered_fields.removed_fields,
        }
    }
}

/// Struct that represents a response of an events request.
#[derive(Debug)]
pub struct EventsCampaignResponse {
    /// The request id as a [String] of the initial events request.
    request_id: String,
    /// A [Receiver] that can be used to receive events.
    pub events_receiver: Receiver<EventEntry>,
}

impl EventsCampaignResponse {
    #[doc(hidden)]
    /// Creates a new `EventsCampaignResponse` object.
    ///
    /// ## Arguments
    ///
    /// * `request_id` - The request id as a [String] for the events request.
    /// * `events_receiver` - A [Receiver<EventEntry>] that can be used to receive events.
    ///
    /// ## Returns
    ///
    /// A new [`EventsCampaignResponse`] object.
    #[must_use]
    pub fn new(request_id: String, events_receiver: Receiver<EventEntry>) -> Self {
        EventsCampaignResponse {
            request_id,
            events_receiver,
        }
    }

    #[doc(hidden)]
    /// Gets the request id.
    ///
    /// ## Returns
    ///
    /// The request id as a [String].
    #[must_use]
    pub fn get_request_id(&self) -> String {
        self.request_id.clone()
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::{EventEntry, EventsCampaignResponse};
    use crate::{
        CompleteState, ankaios_api::ank_base,
        components::complete_state::generate_complete_state_proto,
    };
    use tokio::sync::mpsc;

    const REQUEST_ID: &str = "test_request_id";

    #[test]
    fn utest_events_entry() {
        let proto_entry = ank_base::CompleteStateResponse {
            complete_state: Some(generate_complete_state_proto()),
            altered_fields: Some(ank_base::AlteredFields {
                added_fields: vec!["field1".to_owned()],
                updated_fields: vec!["field2".to_owned()],
                removed_fields: vec!["field3".to_owned()],
            }),
        };
        let event_entry = EventEntry::from(proto_entry);
        assert_eq!(
            event_entry.complete_state,
            CompleteState::new_from_proto(generate_complete_state_proto())
        );
        assert_eq!(event_entry.added_fields, vec!["field1".to_owned()]);
        assert_eq!(event_entry.updated_fields, vec!["field2".to_owned()]);
        assert_eq!(event_entry.removed_fields, vec!["field3".to_owned()]);
    }

    #[test]
    fn utest_events_campaign_response() {
        let (_events_sender, events_receiver) = mpsc::channel(1);
        let events_campaign_response =
            EventsCampaignResponse::new(REQUEST_ID.to_owned(), events_receiver);
        assert_eq!(events_campaign_response.get_request_id(), REQUEST_ID);
    }
}
