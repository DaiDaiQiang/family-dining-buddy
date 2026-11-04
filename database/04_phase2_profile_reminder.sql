-- =====================================================================
--  阶段 2 迁移 · 家庭档案扩展（生日 / 性别 / 忌口）
--  Phase 2 migration - member profile extension
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的列自动跳过
--     mysql -u root -p < database/04_phase2_profile_reminder.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) member.birthday  生日（推荐过滤 + 生日提醒预留）
--    2) member.gender    性别
--    3) member.dislikes  忌口（逗号分隔，推荐过滤用）
--  reminder 表阶段 1 已建好，本阶段应用层启用，无结构变更。
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. member.birthday（幂等加列）
-- ---------------------------------------------------------------------
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'member'
    AND COLUMN_NAME = 'birthday'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `member` ADD COLUMN `birthday` DATE NULL COMMENT ''生日（YYYY-MM-DD，可空）'' AFTER `taste_note`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- ---------------------------------------------------------------------
-- 2. member.gender（幂等加列）
-- ---------------------------------------------------------------------
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'member'
    AND COLUMN_NAME = 'gender'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `member` ADD COLUMN `gender` ENUM(''female'',''male'',''other'') NULL COMMENT ''性别'' AFTER `birthday`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- ---------------------------------------------------------------------
-- 3. member.dislikes（幂等加列）
-- ---------------------------------------------------------------------
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'member'
    AND COLUMN_NAME = 'dislikes'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `member` ADD COLUMN `dislikes` VARCHAR(255) NULL COMMENT ''忌口，逗号分隔（推荐过滤用）'' AFTER `gender`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET FOREIGN_KEY_CHECKS = 1;

-- 迁移完成
SELECT 'phase2 member profile columns ready' AS result;
