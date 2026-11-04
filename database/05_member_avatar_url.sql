-- =====================================================================
-- 迁移 05 · 成员头像图片（相册上传）
--  Phase 5 migration - member avatar image
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的列自动跳过
--     mysql -u root -p < database/05_member_avatar_url.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) member.avatar_url  头像图片地址（/api/uploads/xxx）
--       为空时前端回落 avatar_emoji 显示；设置后优先展示图片。
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. member.avatar_url（幂等加列）
-- ---------------------------------------------------------------------
SET @col_exists = (
  SELECT COUNT(*) FROM information_schema.COLUMNS
  WHERE TABLE_SCHEMA = DATABASE()
    AND TABLE_NAME = 'member'
    AND COLUMN_NAME = 'avatar_url'
);
SET @ddl = IF(@col_exists = 0,
  'ALTER TABLE `member` ADD COLUMN `avatar_url` VARCHAR(255) NULL COMMENT ''头像图片地址（/api/uploads/xxx），空则用 avatar_emoji'' AFTER `avatar_emoji`',
  'SELECT 1');
PREPARE stmt FROM @ddl;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET FOREIGN_KEY_CHECKS = 1;
