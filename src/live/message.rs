use anyhow::{Context, Error, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Local, TimeZone};
use prost::Message;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/iw2.rs"));
}

#[derive(Debug)]
pub struct RawMessage {
    room_id: String,
    data: Value,
}

impl RawMessage {
    pub fn new(room_id: &str, data: Value) -> Self {
        Self {
            room_id: room_id.into(),
            data,
        }
    }

    pub fn data(&self) -> &Value {
        &self.data
    }

    pub fn display(&self) {
        println!(
            "{}",
            serde_json::to_string_pretty(&self.data).expect("failed to serialize")
        );
    }
}
impl RawMessage {
    pub fn msg_type(&self) -> &str {
        self.data["cmd"].as_str().expect("wtf??")
    }
}

impl Display for RawMessage {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.data)
    }
}

impl Deref for RawMessage {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Into<Value> for RawMessage {
    fn into(self) -> Value {
        self.data
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
pub struct Timestamp {
    ts: u64,
    from_server: bool,
}

impl Timestamp {
    fn new_server(timestamp: Option<u64>) -> Result<Self> {
        const THRESHOLD: u64 = 1_000_000_000_000; // 2001-09-09 09:46:40

        let timestamp = timestamp.required("timestamp")?;
        let ts = if timestamp < THRESHOLD {
            timestamp * 1000
        } else {
            timestamp
        };

        Ok(Self {
            ts,
            from_server: true,
        })
    }

    fn new_local() -> Self {
        Self {
            ts: Local::now().timestamp_millis() as u64,
            from_server: false,
        }
    }
}

impl Display for Timestamp {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            fmt,
            "{}{}",
            self.ts,
            if self.from_server { "S" } else { "C" }
        )
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

    fn from_uinfo(uinfo: &Value, wealth_level: Option<i64>) -> Result<Self> {
        Self::new(
            uinfo["uid"].as_u64(),
            uinfo["base"]["name"].as_str(),
            uinfo["base"]["face"].as_str(),
            uinfo["medal"]["level"].as_i64(),
            uinfo["medal"]["score"].as_i64(),
            wealth_level,
        )
    }
}

#[derive(Debug)]
pub enum BattleStatus {
    Start,
    Process,
    End,
}

#[derive(Debug)]
pub enum UserInteractType {
    JoinRoom,
    Subscribe,
    Share,
}

#[derive(Debug)]
pub enum LiveMessage {
    StreamStart {
        // 开播
        timestamp: Timestamp,
    },
    SteamEnd {
        // 停播
        timestamp: Timestamp,
    },
    Danmaku {
        // 弹幕消息
        timestamp: Timestamp,           // 时间戳
        user: UserInfo,                 // 用户信息
        text: String,                   // 消息内容
        extra: HashMap<String, String>, // 附加信息
    },
    SuperChat {
        // 醒目留言
        timestamp: Timestamp, // 时间戳
        user: UserInfo,       // 用户信息
        price: i64,           // 价格
        text: String,         // 留言内容
    },
    Gift {
        // 礼物消息
        timestamp: Timestamp,      // 时间戳
        user: UserInfo,            // 用户信息
        gift_name: String,         // 礼物名称
        gift_count: i64,           // 礼物数量
        coin_type: String,         // 代币类型
        total_coin: i64,           // 礼物金额
        img_basic: Option<String>, // 礼物图片
        img_webp: Option<String>,  // 礼物图片（webp）
    },
    Like {
        // 点赞
        timestamp: Timestamp, // 时间戳
        user: UserInfo,       // 用户信息
    },
    BattleInfo {
        // PK 消息
        timestamp: Timestamp, // 时间戳
        status: BattleStatus,
        opponent_room: String, // 对手房间 ID
        host_votes: i64,       // 本房主播得票
        opponent_votes: i64,   // 对方主播得票
    },
    UserInteract {
        // 进房/关注/分享
        timestamp: Timestamp,       // 时间戳
        user: UserInfo,             // 用户信息
        msg_type: UserInteractType, // 事件类型
    },
    WatchedChange {
        // 历史观众数量变化
        timestamp: Timestamp, // 时间戳
        count: i64,           // 观看数 uv
    },
    Unsupported(String),
}

impl LiveMessage {
    fn timestamp(&self) -> Option<DateTime<Local>> {
        let timestamp = match self {
            LiveMessage::StreamStart { timestamp, .. } => Some(timestamp),
            LiveMessage::SteamEnd { timestamp, .. } => Some(timestamp),
            LiveMessage::Danmaku { timestamp, .. } => Some(timestamp),
            LiveMessage::SuperChat { timestamp, .. } => Some(timestamp),
            LiveMessage::Gift { timestamp, .. } => Some(timestamp),
            LiveMessage::Like { timestamp, .. } => Some(timestamp),
            LiveMessage::BattleInfo { timestamp, .. } => Some(timestamp),
            LiveMessage::UserInteract { timestamp, .. } => Some(timestamp),
            LiveMessage::WatchedChange { timestamp, .. } => Some(timestamp),
            LiveMessage::Unsupported(_) => None,
        };

        timestamp.and_then(|ts| Local.timestamp_micros(ts.ts as i64).single())
    }
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
            "LIVE" => Ok(LiveMessage::StreamStart {
                timestamp: Timestamp::new_server(message["live_time"].as_u64())?,
            }),
            "PREPARING" => Ok(LiveMessage::SteamEnd {
                timestamp: Timestamp::new_server(message["send_time"].as_u64())?,
            }),
            "DANMU_MSG" => {
                let common_data = &message["info"][0][15];

                if common_data.is_null() {
                    bail!("failed to parse common data")
                }

                let common_data_extra: Option<Value> = common_data["extra"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok());

                let mut extra: HashMap<String, String> = HashMap::new();

                if let Some(common_data_extra) = &common_data_extra {
                    let emots = &common_data_extra["emots"];

                    if !emots.is_null() {
                        extra.insert("emots".into(), serde_json::to_string(emots)?);
                    }
                }

                Ok(Self::Danmaku {
                    timestamp: Timestamp::new_server(message["info"][0][4].as_u64())?,
                    user: UserInfo::from_uinfo(
                        &common_data["user"],
                        message["info"][16][0].as_i64(),
                    )?,
                    text: message["info"][1].as_str().required("danmaku text")?.into(),
                    extra,
                })
            }
            "SUPER_CHAT_MESSAGE" => Ok(LiveMessage::SuperChat {
                timestamp: Timestamp::new_server(message["data"]["ts"].as_u64())?,
                user: UserInfo::from_uinfo(&message["data"]["uinfo"], None)?,
                price: message["data"]["price"]
                    .as_i64()
                    .required("super chat price")?,
                text: message["data"]["message"]
                    .as_str()
                    .required("super chat message")?
                    .into(),
            }),
            "SEND_GIFT" => Ok(Self::Gift {
                timestamp: Timestamp::new_server(message["data"]["timestamp"].as_u64())?,
                user: UserInfo::from_uinfo(
                    &message["data"]["sender_uinfo"],
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
            "LIKE_INFO_V3_CLICK" => {
                Ok(Self::Like {
                    timestamp: Timestamp::new_local(),
                    user: UserInfo::from_uinfo(&message["data"]["uinfo"], None)?,
                })
            }
            "PK_BATTLE_START_NEW" | "PK_BATTLE_PROCESS_NEW" | "PK_BATTLE_SETTLE_NEW" => {
                let status = match message.msg_type() {
                    "PK_BATTLE_START_NEW" => BattleStatus::Start,
                    "PK_BATTLE_PROCESS_NEW" => BattleStatus::Process,
                    "PK_BATTLE_SETTLE_NEW" => BattleStatus::End,
                    _ => unreachable!("wtf??"),
                };

                let room_a = message["data"]["init_info"]["room_id"]
                    .as_u64()
                    .required("init_info room")?
                    .to_string();

                let room_b = message["data"]["match_info"]["room_id"]
                    .as_u64()
                    .required("match_info room")?
                    .to_string();

                let vote_a = message["data"]["init_info"]["votes"].as_i64().unwrap_or(0);

                let vote_b = message["data"]["match_info"]["votes"].as_i64().unwrap_or(0);

                let (opponent_room, host_votes, opponent_votes) = {
                    if room_a == message.room_id {
                        (room_b, vote_a, vote_b)
                    } else {
                        (room_a, vote_b, vote_a)
                    }
                };

                Ok(Self::BattleInfo {
                    timestamp: Timestamp::new_server(message["timestamp"].as_u64())?,
                    status,
                    opponent_room,
                    host_votes,
                    opponent_votes,
                })
            }
            "INTERACT_WORD_V2" => {
                let pb_data = message["data"]["pb"].as_str().required("pb")?;
                let pb_data = STANDARD.decode(pb_data.as_bytes())?;

                let iw2 = proto::InteractWordV2::decode(pb_data.as_slice())?;

                Ok(LiveMessage::UserInteract {
                    timestamp: Timestamp::new_server(Some(iw2.timestamp)).expect("wtf??"),
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
            "WATCHED_CHANGE" => Ok(Self::WatchedChange {
                timestamp: Timestamp::new_local(),
                count: message["data"]["num"].as_i64().required("watched count")?,
            }),
            _ => Ok(Self::Unsupported(message.msg_type().into())),
        }
    }
}
