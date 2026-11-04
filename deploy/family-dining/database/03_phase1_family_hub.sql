-- =====================================================================
--  阶段 1 迁移 · 家庭管理底座（账本 / 快递 / 健康 / 学习同步 / 提醒 / 鉴权）
--  Phase 1 migration - family management foundation
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的列/表自动跳过
--     mysql -u root -p < database/03_phase1_family_hub.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) member 表增加 password_hash（每人密码，NULL=未设置可免密登录）
--    2) 新增 6 张表 : ledger_entry / parcel / health_profile /
--                     member_data / app_setting / reminder
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. member.password_hash（幂等加列）
-- ---------------------------------------------------------------------
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'member'
    AND COLUMN_NAME = 'password_hash'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `member` ADD COLUMN `password_hash` VARCHAR(255) NULL COMMENT ''密码哈希（argon2），NULL=未设置可免密登录'' AFTER `taste_note`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- ---------------------------------------------------------------------
-- 2. 账本（收支记录）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `ledger_entry` (
  `id`          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '记录ID',
  `member_id`   BIGINT UNSIGNED NOT NULL                COMMENT '记账人',
  `type`        ENUM('income','expense') NOT NULL DEFAULT 'expense' COMMENT '收支类型',
  `category`    VARCHAR(32)     NOT NULL                COMMENT '分类：买菜/购物/外食/日用/交通/医疗/育儿/居住/娱乐/工资/奖金红包/其他',
  `amount`      DECIMAL(12,2)   NOT NULL                COMMENT '金额（元）',
  `entry_date`  DATE            NOT NULL                COMMENT '发生日期',
  `note`        VARCHAR(255)    NULL                    COMMENT '备注',
  `created_at`  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_ledger_date` (`entry_date`),
  KEY `idx_ledger_member` (`member_id`),
  KEY `idx_ledger_type_date` (`type`, `entry_date`),
  CONSTRAINT `fk_ledger_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `chk_ledger_type` CHECK (`type` IN ('income','expense')),
  CONSTRAINT `chk_ledger_amount` CHECK (`amount` >= 0)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='家庭账本';

-- ---------------------------------------------------------------------
-- 3. 快递取件码
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `parcel` (
  `id`                   BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '快递ID',
  `pickup_code`          VARCHAR(64)     NOT NULL                COMMENT '取件码',
  `courier_code`         VARCHAR(32)     NOT NULL DEFAULT 'other' COMMENT '快递公司编码（快递100 com 码，为物流轨迹预留）',
  `courier_name`         VARCHAR(32)     NULL                    COMMENT '快递公司显示名',
  `station_type`         VARCHAR(32)     NULL                    COMMENT '驿站类型：菜鸟/妈妈驿站/丰巢/驿站/快递柜/其他',
  `station`              VARCHAR(64)     NULL                    COMMENT '驿站/自提点名称',
  `shelf_no`             VARCHAR(32)     NULL                    COMMENT '货架号',
  `tracking_no`          VARCHAR(64)     NULL                    COMMENT '运单号',
  `note`                 VARCHAR(255)    NULL                    COMMENT '备注',
  `recipient_member_id`  BIGINT UNSIGNED NULL                    COMMENT '收件人（成员ID）',
  `recipient_name`       VARCHAR(32)     NULL                    COMMENT '收件人姓名快照',
  `arrived_date`         DATE            NOT NULL                COMMENT '到件日期',
  `status`               ENUM('pending','picked') NOT NULL DEFAULT 'pending' COMMENT '状态：待取/已取',
  `picked_at`            DATETIME        NULL                    COMMENT '取件时间',
  `created_at`           DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`           DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_parcel_status_date` (`status`, `arrived_date`),
  KEY `idx_parcel_recipient` (`recipient_member_id`),
  CONSTRAINT `fk_parcel_recipient` FOREIGN KEY (`recipient_member_id`) REFERENCES `member`(`id`)
    ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT `chk_parcel_status` CHECK (`status` IN ('pending','picked'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='快递取件码';

-- 兼容已建表环境：补 station 列（幂等）
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'parcel'
    AND COLUMN_NAME = 'station'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `parcel` ADD COLUMN `station` VARCHAR(64) NULL COMMENT ''驿站/自提点名称'' AFTER `station_type`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- ---------------------------------------------------------------------
-- 4. 健康档案（经期周期，一成员一行）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `health_profile` (
  `id`                BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '档案ID',
  `member_id`         BIGINT UNSIGNED NOT NULL                COMMENT '所属成员',
  `gender`            ENUM('female','male','other') NULL      COMMENT '性别',
  `last_period_start` DATE            NULL                    COMMENT '上次月经开始日',
  `cycle_length`      TINYINT UNSIGNED NOT NULL DEFAULT 28    COMMENT '周期长度（天）',
  `period_length`     TINYINT UNSIGNED NOT NULL DEFAULT 5     COMMENT '经期长度（天）',
  `created_at`        DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`        DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_health_member` (`member_id`),
  CONSTRAINT `fk_health_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `chk_health_cycle`  CHECK (`cycle_length` BETWEEN 15 AND 60),
  CONSTRAINT `chk_health_period` CHECK (`period_length` BETWEEN 1 AND 15)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='成员健康档案';

-- ---------------------------------------------------------------------
-- 5. 成员同步数据（JSON 整包存储：学习模块等）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `member_data` (
  `id`         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `member_id`  BIGINT UNSIGNED NOT NULL                COMMENT '所属成员',
  `data_key`   VARCHAR(32)     NOT NULL                COMMENT '数据域：study 等',
  `payload`    JSON            NOT NULL                COMMENT '整包数据（含 version 字段由应用层维护）',
  `version`    INT UNSIGNED    NOT NULL DEFAULT 1      COMMENT '乐观版本号',
  `updated_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_member_data` (`member_id`, `data_key`),
  CONSTRAINT `fk_member_data_owner` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='成员同步数据（JSON blob）';

-- ---------------------------------------------------------------------
-- 6. 应用设置（KV）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `app_setting` (
  `setting_key`   VARCHAR(64)  NOT NULL COMMENT '设置键',
  `setting_value` VARCHAR(1024) NOT NULL COMMENT '设置值',
  `updated_at`    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`setting_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='应用设置';

-- ---------------------------------------------------------------------
-- 7. 提醒中心（阶段 2 启用，结构先行）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `reminder` (
  `id`         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '提醒ID',
  `member_id`  BIGINT UNSIGNED NULL                    COMMENT '归属成员（NULL=全家）',
  `type`       ENUM('health','parcel','bill','device','other') NOT NULL COMMENT '类型',
  `title`      VARCHAR(128)    NOT NULL                COMMENT '提醒标题',
  `due_at`     DATETIME        NOT NULL                COMMENT '提醒时间',
  `status`     ENUM('pending','done','dismissed') NOT NULL DEFAULT 'pending' COMMENT '状态',
  `payload`    JSON            NULL                    COMMENT '附加数据',
  `created_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_reminder_due` (`status`, `due_at`),
  KEY `idx_reminder_member` (`member_id`),
  CONSTRAINT `fk_reminder_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `chk_reminder_type`   CHECK (`type` IN ('health','parcel','bill','device','other')),
  CONSTRAINT `chk_reminder_status` CHECK (`status` IN ('pending','done','dismissed'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='提醒中心';

SET FOREIGN_KEY_CHECKS = 1;
