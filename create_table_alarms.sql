-- DDL Tabel Alarm ClickHouse untuk Telkomsel FBB Alarm Pipeline
-- Database: telkomsel atau default

CREATE DATABASE IF NOT EXISTS telkomsel;

CREATE TABLE IF NOT EXISTS telkomsel.active_alarms
(
    -- Identitas & Primary Key
    identifier String COMMENT 'Unique alarm hash identifier (100% populated)',
    ticketid Nullable(String) COMMENT 'Incident ticket ID (SM-...)',
    alarmid String COMMENT 'Alarm ID tracking',
    alarmserialnumber String COMMENT 'Hardware EMS alarm serial number',

    -- Status & Filter Kritis
    cleartime Int64 COMMENT '0 = Alarm Aktif, >0 = Alarm Cleared (Epoch ms)',
    is_active UInt8 MATERIALIZED (cleartime = 0 ? 1 : 0),
    severity UInt8 COMMENT 'Severity level: 1=Indeterminate, 2=Warning/Low, 3=Minor, 4=Major, 5=Critical',
    ticketstatus Int32 COMMENT 'Ticket lifecycle status',

    -- Informasi Alarm & Perangkat
    alarmname LowCardinality(String) COMMENT 'Alarm name (e.g. Multiple ONT Down, LOS)',
    alarmtext String COMMENT 'Detailed alarm message',
    alarmtype LowCardinality(String) COMMENT 'Type of alarm',
    domain Int32,
    summary String COMMENT 'Alarm summary description',

    -- Topologi & Lokasi Jaringan
    location LowCardinality(String) COMMENT 'Regional office e.g. REGIONAL1, REGIONAL5',
    area LowCardinality(String) COMMENT 'Area operation',
    sitecode LowCardinality(String) COMMENT 'Site code e.g. RAP, KND, SBY',
    siteid LowCardinality(String),
    sitename String,
    node LowCardinality(String) COMMENT 'Network Node / OLT / Device name e.g. GPON00-D1-f',
    devicename String,
    ipaddress String,
    vendor LowCardinality(String),

    -- Timestamp & Durasi
    firstoccurrence Int64 COMMENT 'First occurrence in Unix Milliseconds',
    lastoccurrence Int64 COMMENT 'Last occurrence in Unix Milliseconds',
    alarm_start DateTime('Asia/Jakarta') COMMENT 'Alarm start datetime in Jakarta timezone',
    cleared_time_dt Nullable(DateTime('Asia/Jakarta')),
    duration_seconds UInt32 MATERIALIZED (cleartime = 0 ? toUInt32(now() - alarm_start) : toUInt32(ifNull(cleared_time_dt, alarm_start) - alarm_start)),

    -- Metadata Tambahan & Zero Data Loss
    acknowledged LowCardinality(String) DEFAULT '0',
    totalcount UInt32 DEFAULT 1,
    eventtype LowCardinality(String) DEFAULT '',
    raw_attributes String COMMENT 'Seluruh 273 atribut mentah dalam format JSON (Zero Data Loss)',
    ingested_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(lastoccurrence)
PARTITION BY toYYYYMM(alarm_start)
ORDER BY (location, severity, alarm_start, identifier);

-- Tabel cadangan di database default jika aplikasi terhubung ke database default
CREATE TABLE IF NOT EXISTS default.active_alarms AS telkomsel.active_alarms;
