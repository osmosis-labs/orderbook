use osmosis_std_derive::CosmwasmExt;

#[derive(
    Clone,
    PartialEq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/cosmos.bank.v1beta1.MsgSend")]
pub struct MsgSend {
    #[prost(string, tag = "1")]
    pub from_address: String,
    #[prost(string, tag = "2")]
    pub to_address: String,
    #[prost(message, repeated, tag = "3")]
    pub amount: ::prost::alloc::vec::Vec<osmosis_std::types::cosmos::base::v1beta1::Coin>,
}
