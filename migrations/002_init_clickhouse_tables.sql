-- Migration 002: Initialize ClickHouse OLAP analytics and active alarm tables
-- Target engine: ClickHouse 24.8+

CREATE DATABASE IF NOT EXISTS telkomsel;

CREATE TABLE IF NOT EXISTS default.user_activities (
    user_id UInt64,
    action String,
    details String,
    timestamp UInt64
)
ENGINE = MergeTree()
ORDER BY (user_id, timestamp);

CREATE TABLE IF NOT EXISTS telkomsel.active_alarms (
    identifier String COMMENT 'Unique alarm hash identifier',
    ticketid Nullable(String) COMMENT 'Incident ticket ID',
    alarmid String COMMENT 'Alarm tracking identifier',
    alarmserialnumber String COMMENT 'Hardware EMS alarm serial number',
    cleartime Int64 COMMENT '0 = active alarm, >0 = cleared epoch ms',
    is_active UInt8 MATERIALIZED (cleartime = 0 ? 1 : 0),
    severity UInt8 COMMENT '1=Indeterminate, 2=Warning, 3=Minor, 4=Major, 5=Critical',
    ticketstatus Int32 COMMENT 'Ticket lifecycle status',
    alarmname LowCardinality(String),
    alarmtext String,
    alarmtype LowCardinality(String),
    domain Int32,
    summary String,
    location LowCardinality(String),
    area LowCardinality(String),
    sitecode LowCardinality(String),
    siteid LowCardinality(String),
    sitename String,
    node LowCardinality(String),
    devicename String,
    ipaddress String,
    vendor LowCardinality(String),
    firstoccurrence Int64,
    lastoccurrence Int64,
    alarm_start DateTime('Asia/Jakarta'),
    cleared_time_dt Nullable(DateTime('Asia/Jakarta')),
    duration_seconds UInt32 MATERIALIZED (
        cleartime = 0
            ? toUInt32(now() - alarm_start)
            : toUInt32(ifNull(cleared_time_dt, alarm_start) - alarm_start)
    ),
    acknowledged LowCardinality(String) DEFAULT '0',
    totalcount UInt32 DEFAULT 1,
    eventtype LowCardinality(String) DEFAULT '',
    raw_attributes String COMMENT 'Complete raw attributes in JSON format',
    ingested_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(lastoccurrence)
PARTITION BY toYYYYMM(alarm_start)
ORDER BY (location, severity, alarm_start, identifier);

CREATE TABLE IF NOT EXISTS default.active_alarms AS telkomsel.active_alarms;
