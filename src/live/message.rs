use anyhow::{Context, Error, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use prost::Message;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/iw2.rs"));
}

#[derive(Debug)]
pub struct RawMessage(Value);

impl RawMessage {
    pub fn new(data: Value) -> Self {
        Self(data)
    }

    pub fn data(&self) -> &Value {
        &self.0
    }

    pub fn display(&self) {
        println!(
            "{}",
            serde_json::to_string_pretty(&self.0).expect("failed to serialize")
        );
    }
}
impl RawMessage {
    pub fn msg_type(&self) -> &str {
        self.0["cmd"].as_str().expect("wtf??")
    }
}

impl Display for RawMessage {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl Deref for RawMessage {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<Value> for RawMessage {
    fn into(self) -> Value {
        self.0
    }
}

trait Required<T> {
    fn required(self, field: &str) -> Result<T>;
}

impl<T> Required<T> for Option<T> {
    fn required(self, field: &str) -> Result<T> {
        self.context(format!("failed to parse {field}"))
    }
}

#[derive(Debug)]
pub struct UserInfo {
    uid: u64,                  // UID
    uname: String,             // 用户名
    face: Option<String>,      // 头像
    medal_level: Option<i64>,  // 粉丝团等级
    medal_score: Option<i64>,  // 粉丝团积分（亲密度？）
    wealth_level: Option<i64>, // 荣耀等级
}

impl UserInfo {
    fn new<U: AsRef<str>, F: AsRef<str>>(
        uid: Option<u64>,
        uname: Option<U>,
        face: Option<F>,
        medal_level: Option<i64>,
        medal_score: Option<i64>,
        wealth_level: Option<i64>,
    ) -> Result<Self> {
        Ok(Self {
            uid: uid.required("uid")?,
            uname: uname.required("uname")?.as_ref().into(),
            face: face.map(|x| x.as_ref().into()),
            medal_level,
            medal_score,
            wealth_level,
        })
    }
}

#[derive(Debug)]
pub enum UserInteractType {
    JoinRoom,
    Subscribe,
    Share,
}

#[derive(Debug)]
pub enum LiveMessage {
    Danmaku {
        // 弹幕消息
        timestamp: u64,                // 时间戳
        user: UserInfo,                // 用户信息
        text: String,                  // 消息内容
        extra: HashMap<String, Value>, // 附加信息
    },
    Gift {
        // 礼物消息
        timestamp: u64,            // 时间戳
        user: UserInfo,            // 用户信息
        gift_name: String,         // 礼物名称
        gift_count: i64,           // 礼物数量
        coin_type: String,         // 代币类型
        total_coin: i64,           // 礼物金额
        img_basic: Option<String>, // 礼物图片
        img_webp: Option<String>,  // 礼物图片（webp）
    },
    WatchedChange {
        // 历史观众数量变化
        count: i64,
    },
    UserInteract {
        // 进房/关注/分享
        timestamp: u64,             // 时间戳
        user: UserInfo,             // 用户信息
        msg_type: UserInteractType, // 事件类型
    },
    Unsupported(String), // 尚未支持的消息
}

macro_rules! nested_opt {
    ($proto:expr; $target:ident) => {
        Some(&$proto).map(|x| (&x.$target).to_owned())
    };
    ($proto:expr; $($field:ident),*; $target:ident) => {
        Some(&$proto)
            $(.and_then(|x| x.$field.as_ref()))*
            .map(|x| (&x.$target).to_owned())
    };
}

impl TryFrom<RawMessage> for LiveMessage {
    type Error = Error;

    fn try_from(message: RawMessage) -> Result<Self, Self::Error> {
        match message.msg_type() {
            "DANMU_MSG" => {
                let common_data = &message["info"][0][15];

                if common_data.is_null() {
                    bail!("failed to parse common data")
                }

                let common_data_extra: Option<Value> = common_data["extra"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok());

                let mut extra: HashMap<String, Value> = HashMap::new();

                if let Some(common_data_extra) = &common_data_extra {
                    let emots = &common_data_extra["emots"];

                    if !emots.is_null() {
                        extra.insert("emots".into(), Value::clone(emots));
                    }
                }

                Ok(Self::Danmaku {
                    timestamp: message["info"][0][4].as_u64().required("timestamp")?,
                    user: UserInfo::new(
                        common_data["user"]["uid"].as_u64(),
                        common_data["user"]["base"]["name"].as_str(),
                        common_data["user"]["base"]["face"].as_str(),
                        common_data["user"]["medal"]["level"].as_i64(),
                        common_data["user"]["medal"]["score"].as_i64(),
                        message["info"][16][0].as_i64(),
                    )?,
                    text: message["info"][1].as_str().required("danmaku text")?.into(),
                    extra,
                })
            }
            "SEND_GIFT" => Ok(Self::Gift {
                timestamp: message["data"]["timestamp"]
                    .as_u64()
                    .required("timestamp")?,
                user: UserInfo::new(
                    message["data"]["sender_uinfo"]["uid"].as_u64(),
                    message["data"]["sender_uinfo"]["base"]["name"].as_str(),
                    message["data"]["sender_uinfo"]["base"]["face"].as_str(),
                    message["data"]["sender_uinfo"]["medal"]["level"].as_i64(),
                    message["data"]["sender_uinfo"]["medal"]["score"].as_i64(),
                    message["data"]["wealth_level"].as_i64(),
                )?,
                gift_name: message["data"]["giftName"]
                    .as_str()
                    .required("gift name")?
                    .into(),
                gift_count: message["data"]["num"].as_i64().required("gift count")?,
                coin_type: message["data"]["coin_type"]
                    .as_str()
                    .required("coin type")?
                    .into(),
                total_coin: message["data"]["total_coin"]
                    .as_i64()
                    .required("total coin")?,
                img_basic: message["data"]["gift_info"]["img_basic"]
                    .as_str()
                    .map(|x| x.into()),
                img_webp: message["data"]["gift_info"]["webp"]
                    .as_str()
                    .map(|x| x.into()),
            }),
            "WATCHED_CHANGE" => Ok(Self::WatchedChange {
                count: message["data"]["num"].as_i64().required("watched count")?,
            }),
            "INTERACT_WORD_V2" => {
                let pb_data = message["data"]["pb"].as_str().required("pb")?;
                let pb_data = STANDARD.decode(pb_data.as_bytes())?;

                let iw2 = proto::InteractWordV2::decode(pb_data.as_slice())?;

                Ok(LiveMessage::UserInteract {
                    timestamp: iw2.timestamp,
                    user: UserInfo::new(
                        Some(iw2.uid),
                        nested_opt!(iw2; uname),
                        nested_opt!(iw2; uinfo, base; face),
                        nested_opt!(iw2; fans_medal; medal_level),
                        nested_opt!(iw2; fans_medal; score),
                        nested_opt!(iw2; uinfo, wealth; level),
                    )?,
                    msg_type: {
                        match iw2.msg_type {
                            1 => UserInteractType::JoinRoom,
                            2 => UserInteractType::Subscribe,
                            3 => UserInteractType::Share,
                            _ => bail!("unknown message type"),
                        }
                    },
                })
            }
            _ => Ok(Self::Unsupported(message.msg_type().into())),
        }
    }
}
