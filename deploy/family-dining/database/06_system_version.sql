-- =====================================================================
--  迁移 06 · 系统版本更新记录
--  System version changelog
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+
--  执行方式 : 可重复执行（幂等）——已存在的表/种子数据自动跳过
--     mysql -u root -p < database/06_system_version.sql
--     或 cd database/tools && npm run migrate
--
--  变更内容 :
--    1) system_version  版本更新记录表（版本号 + 更新内容，每次发版追加一条）
--    2) 种子数据        当前 1.0 版本的首条记录
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. system_version（版本更新记录）
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS `system_version` (
  `id`          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '记录ID',
  `version`     VARCHAR(32)     NOT NULL                COMMENT '版本号，如 1.0',
  `title`       VARCHAR(128)    NULL                    COMMENT '更新主题（可选）',
  `content`     TEXT            NOT NULL                COMMENT '更新内容',
  `released_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '发布时间',
  `created_at`  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_system_version` (`version`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='系统版本更新记录';

-- ---------------------------------------------------------------------
-- 2. 种子数据：当前 1.0 版本（幂等，已存在则跳过）
-- ---------------------------------------------------------------------
INSERT INTO `system_version` (`version`, `title`, `content`, `released_at`)
SELECT '1.0', '系统首个正式版本',
       '家庭管理系统 1.0 正式上线：\n- 今天吃什么：菜谱库、点餐下单、订单管理\n- 常用功能：家庭账本、快递取件码、经期提醒、提醒中心、学习园地\n- 家的味道：消费与口味统计看板\n- 个人中心：头像、密码、家庭成员管理\n- 系统版本：版本号与更新内容记录（就是你在看的这页）',
       NOW()
FROM DUAL
WHERE NOT EXISTS (SELECT 1 FROM `system_version` WHERE `version` = '1.0');

SET FOREIGN_KEY_CHECKS = 1;

-- 迁移完成
SELECT 'system_version ready' AS result;
