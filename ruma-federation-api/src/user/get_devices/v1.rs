//! [GET /_matrix/federation/v1/user/devices/](https://matrix.org/docs/spec/server_server/r0.1.4#get-matrix-federation-v1-user-devices-userid)

use ruma_api::ruma_api;
use ruma_client_api::r0::Device;
use ruma_identifiers::{DeviceId, UserId};

ruma_api! {
    metadata: {
        description: "Gets information on all of the user's devices.",
        name: "get_devices",
        method: GET,
        path: "/_matrix/federation/v1/user/devices",
        rate_limited: false,
        requires_authentication: true,
    }

    request: {
        /// The user ID to retrieve devices for. Must be a user local to the receiving homeserver.
        #[ruma_api(query)]
        pub user_id: UserId,
    }

    response: {
        /// The user ID devices were requested for.
        pub user_id: UserId,

        /// A unique ID for a given user_id which describes the version of the returned device
        /// list. This is matched with the `stream_id` field in `m.device_list_update` EDUs in
        /// order to incrementally update the returned device_list.
        pub stream_id: u64,

        /// The user's devices. May be empty.
        pub devices: Vec<Device>,
    }
}
