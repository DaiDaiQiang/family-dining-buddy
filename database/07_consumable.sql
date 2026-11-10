-- =====================================================================
--  阶段 3 迁移 · 耗材维护与消耗统计
--  Phase 3 migration - consumable (supply) maintenance & usage stats
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的表自动跳过
--     mysql -u root -p < database/07_consumable.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) consumable      耗材档案（名称 / 单位 / 当前库存 / 预警阈值 / 分类 / 图标）
--    2) consumable_log  消耗记录（每次登记一条，含数量 / 去向 / 登记人 / 登记时间）
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. consumable 耗材档案
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `consumable` (
  `id`            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `name`          VARCHAR(64)     NOT NULL COMMENT '耗材名称，如 洗洁精',
  `category`      VARCHAR(32)     NOT NULL DEFAULT '其他' COMMENT '分类：清洁/纸品/调味料/其他',
  `unit`          VARCHAR(16)     NOT NULL DEFAULT '瓶' COMMENT '计量单位：瓶/包/卷/个',
  `emoji`         VARCHAR(8)      NOT NULL DEFAULT '🧴' COMMENT '图标 emoji',
  `stock`         DECIMAL(10,2)   NOT NULL DEFAULT 0 COMMENT '当前库存量',
  `low_threshold` DECIMAL(10,2)   NOT NULL DEFAULT 0 COMMENT '低于此值触发补货提醒',
  `note`          VARCHAR(255)    NULL COMMENT '备注（品牌/规格等）',
  `is_active`     TINYINT(1)      NOT NULL DEFAULT 1 COMMENT '是否启用',
  `created_at`    DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`    DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_consumable_active` (`is_active`),
  KEY `idx_consumable_name` (`name`)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci COMMENT = '家庭耗材档案';

-- ---------------------------------------------------------------------
-- 2. consumable_log 消耗记录
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `consumable_log` (
  `id`            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `consumable_id` BIGINT UNSIGNED NOT NULL COMMENT '耗材 id',
  `quantity`      DECIMAL(10,2)   NOT NULL COMMENT '本次消耗量（正数）',
  `member_id`     BIGINT UNSIGNED NULL COMMENT '登记人（成员 id，可空）',
  `member_name`   VARCHAR(64)     NULL COMMENT '登记人昵称快照',
  `note`          VARCHAR(255)    NULL COMMENT '用途备注（如 洗碗）',
  `occurred_at`   DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '消耗时间',
  `created_at`    DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  KEY `idx_clog_consumable` (`consumable_id`),
  KEY `idx_clog_occurred` (`occurred_at`),
  CONSTRAINT `fk_clog_consumable` FOREIGN KEY (`consumable_id`)
    REFERENCES `consumable` (`id`) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci COMMENT = '耗材消耗记录';

SET FOREIGN_KEY_CHECKS = 1;

-- ---------------------------------------------------------------------
-- 3. reminder.type 枚举补 'consumable'（低库存提醒）
-- ---------------------------------------------------------------------
ALTER TABLE `reminder`
  MODIFY COLUMN `type` ENUM('health','parcel','bill','device','consumable','other') NOT NULL COMMENT '类型',
  DROP CHECK `chk_reminder_type`,
  ADD CONSTRAINT `chk_reminder_type` CHECK (`type` IN ('health','parcel','bill','device','consumable','other'));

-- 迁移完成
SELECT 'consumable tables ready' AS result;
