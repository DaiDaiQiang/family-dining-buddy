-- =====================================================================
--  学习模块扩展迁移 · 家庭共享数据（公有题库）
--  Family-shared data (public study banks)
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的表自动跳过
--     mysql -u root -p < database/08_shared_data.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) shared_data  家庭级共享 JSON blob（无 member 外键，全家共享一份）
--       用途：学习模块的「公有题库」——所有家庭成员可见可刷，
--       只有创建者（记录在 payload 内）可改 / 删。
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. shared_data 家庭共享数据
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `shared_data` (
  `id`         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `data_key`   VARCHAR(32)     NOT NULL                COMMENT '数据域：study_shared 等',
  `payload`    JSON            NOT NULL                COMMENT '整包数据（含创建者等元信息）',
  `version`    INT UNSIGNED    NOT NULL DEFAULT 1      COMMENT '乐观版本号',
  `updated_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_shared_data` (`data_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='家庭共享数据（JSON blob，如公有题库）';

SET FOREIGN_KEY_CHECKS = 1;
