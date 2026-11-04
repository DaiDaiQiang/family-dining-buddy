-- =====================================================================
--  家庭点餐系统 · 数据库架构
--  Home Family Dining System - Database Schema
-- ---------------------------------------------------------------------
--  目标版本 : MySQL 8.0.16+（使用 CHECK 约束、CTE、窗口函数、utf8mb4_0900）
--  字符集   : utf8mb4 / utf8mb4_0900_ai_ci（完整支持中文与 emoji）
--  存储引擎 : InnoDB（事务 + 外键）
--
--  导入方式 :
--     mysql -u root -p < database/01_schema.sql
--     mysql -u root -p < database/02_seed_data.sql
--
--  说明：本脚本使用 DELIMITER，请通过 mysql 命令行客户端执行，
--        不要粘贴到不支持 DELIMITER 的简易客户端里。
-- =====================================================================

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;
SET SQL_MODE = 'STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION';

-- ---------------------------------------------------------------------
-- 0. 数据库
-- ---------------------------------------------------------------------
CREATE DATABASE IF NOT EXISTS `family_dining`
  DEFAULT CHARACTER SET utf8mb4
  DEFAULT COLLATE utf8mb4_0900_ai_ci;

USE `family_dining`;

-- ---------------------------------------------------------------------
-- 1. 清理旧对象（便于重复执行本脚本重建）
-- ---------------------------------------------------------------------
DROP PROCEDURE IF EXISTS `sp_generate_shopping_list`;
DROP PROCEDURE IF EXISTS `sp_confirm_order`;
DROP VIEW IF EXISTS `v_member_taste_stat`;
DROP VIEW IF EXISTS `v_category_stat`;
DROP VIEW IF EXISTS `v_recipe_full`;
DROP VIEW IF EXISTS `v_order_shopping_preview`;
DROP VIEW IF EXISTS `v_order_detail`;

DROP TABLE IF EXISTS `shopping_item`;
DROP TABLE IF EXISTS `order_item`;
DROP TABLE IF EXISTS `meal_order`;
DROP TABLE IF EXISTS `member_favorite`;
DROP TABLE IF EXISTS `recipe_tag`;
DROP TABLE IF EXISTS `tag`;
DROP TABLE IF EXISTS `recipe_ingredient`;
DROP TABLE IF EXISTS `ingredient`;
DROP TABLE IF EXISTS `recipe_step`;
DROP TABLE IF EXISTS `recipe`;
DROP TABLE IF EXISTS `member`;

-- =====================================================================
--  2. 表结构
-- =====================================================================

-- ---------------------------------------------------------------------
-- 2.1 家庭成员 —— 轻量昵称制，免密码
-- ---------------------------------------------------------------------
CREATE TABLE `member` (
  `id`            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '成员ID',
  `nickname`      VARCHAR(32)     NOT NULL                COMMENT '昵称，如 爸爸/妈妈/宝宝',
  `avatar_emoji`  VARCHAR(16)     NOT NULL DEFAULT '🙂'    COMMENT '头像 emoji',
  `theme_color`   CHAR(7)         NOT NULL DEFAULT '#FF8A65' COMMENT '成员主题色（前端头像底色）',
  `role`          ENUM('admin','member','child') NOT NULL DEFAULT 'member' COMMENT '家庭角色',
  `taste_note`    VARCHAR(255)    NULL                    COMMENT '口味备注，如 不吃香菜、少辣',
  `is_active`     TINYINT(1)      NOT NULL DEFAULT 1      COMMENT '是否启用：1启用 0停用',
  `sort_order`    INT             NOT NULL DEFAULT 0      COMMENT '显示排序',
  `created_at`    DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`    DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_member_nickname` (`nickname`),
  KEY `idx_member_active_sort` (`is_active`, `sort_order`),
  CONSTRAINT `chk_member_role` CHECK (`role` IN ('admin','member','child')),
  CONSTRAINT `chk_member_color` CHECK (`theme_color` REGEXP '^#[0-9A-Fa-f]{6}$')
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='家庭成员';

-- ---------------------------------------------------------------------
-- 2.2 菜谱主表
--     cover_emoji + cover_gradient 用于免图床的漂亮封面，
--     cover_url 可选（有真实图片时使用）
-- ---------------------------------------------------------------------
CREATE TABLE `recipe` (
  `id`              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT COMMENT '菜谱ID',
  `name`            VARCHAR(64)     NOT NULL                COMMENT '菜名',
  `category`        VARCHAR(32)     NOT NULL DEFAULT '家常菜' COMMENT '分类：家常菜/汤羹/主食/凉菜/甜点/饮品',
  `cuisine`         VARCHAR(32)     NOT NULL DEFAULT '中餐'  COMMENT '菜系：中餐/川菜/粤菜/西餐/日韩/东南亚',
  `difficulty`      ENUM('easy','medium','hard') NOT NULL DEFAULT 'easy' COMMENT '难度',
  `prep_minutes`    SMALLINT UNSIGNED NOT NULL DEFAULT 10   COMMENT '备菜时长（分钟）',
  `cook_minutes`    SMALLINT UNSIGNED NOT NULL DEFAULT 15   COMMENT '烹饪时长（分钟）',
  `servings`        TINYINT UNSIGNED NOT NULL DEFAULT 2     COMMENT '默认份数（用量换算基准）',
  `total_minutes`   SMALLINT UNSIGNED GENERATED ALWAYS AS (`prep_minutes` + `cook_minutes`) STORED COMMENT '总时长（生成列）',
  `cover_emoji`     VARCHAR(16)     NOT NULL DEFAULT '🍲'    COMMENT '封面 emoji',
  `cover_gradient`  VARCHAR(64)     NOT NULL DEFAULT 'linear-gradient(135deg,#FF9A8B,#FF6A88)' COMMENT '封面渐变色',
  `cover_url`       VARCHAR(512)    NULL                    COMMENT '封面图片URL（可选）',
  `description`     VARCHAR(512)    NULL                    COMMENT '一句话简介',
  `tips`            TEXT            NULL                    COMMENT '小贴士',
  `calories`        SMALLINT UNSIGNED NULL                  COMMENT '每份估算热量(kcal)',
  `is_active`       TINYINT(1)      NOT NULL DEFAULT 1      COMMENT '是否上架：1可点 0隐藏',
  `cook_count`      INT UNSIGNED    NOT NULL DEFAULT 0      COMMENT '累计点单次数（冗余统计）',
  `avg_rating`      DECIMAL(3,2)    NOT NULL DEFAULT 0.00   COMMENT '平均评分（冗余统计）',
  `created_by`      BIGINT UNSIGNED NULL                    COMMENT '创建人（成员ID）',
  `created_at`      DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`      DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_recipe_name` (`name`),
  KEY `idx_recipe_active_category` (`is_active`, `category`),
  KEY `idx_recipe_cuisine` (`cuisine`),
  KEY `idx_recipe_rating` (`avg_rating` DESC, `cook_count` DESC),
  KEY `idx_recipe_created_by` (`created_by`),
  FULLTEXT KEY `ft_recipe_name_desc` (`name`, `description`) WITH PARSER ngram,
  CONSTRAINT `fk_recipe_creator` FOREIGN KEY (`created_by`) REFERENCES `member`(`id`)
    ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT `chk_recipe_difficulty` CHECK (`difficulty` IN ('easy','medium','hard')),
  CONSTRAINT `chk_recipe_servings`   CHECK (`servings` BETWEEN 1 AND 20),
  CONSTRAINT `chk_recipe_minutes`    CHECK (`prep_minutes` <= 600 AND `cook_minutes` <= 600),
  CONSTRAINT `chk_recipe_rating`     CHECK (`avg_rating` BETWEEN 0 AND 5)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='菜谱';

-- ---------------------------------------------------------------------
-- 2.3 菜谱做法步骤（独立表，便于分步展示）
-- ---------------------------------------------------------------------
CREATE TABLE `recipe_step` (
  `id`           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `recipe_id`    BIGINT UNSIGNED NOT NULL COMMENT '所属菜谱',
  `step_no`      SMALLINT UNSIGNED NOT NULL COMMENT '步骤序号，从1开始',
  `content`      VARCHAR(1000)   NOT NULL COMMENT '步骤说明',
  `duration_sec` SMALLINT UNSIGNED NULL   COMMENT '该步骤耗时（秒），可空',
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_step_recipe_no` (`recipe_id`, `step_no`),
  CONSTRAINT `fk_step_recipe` FOREIGN KEY (`recipe_id`) REFERENCES `recipe`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='菜谱做法步骤';

-- ---------------------------------------------------------------------
-- 2.4 食材库（全局字典，采购清单聚合的基础）
-- ---------------------------------------------------------------------
CREATE TABLE `ingredient` (
  `id`         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `name`       VARCHAR(64)     NOT NULL COMMENT '食材名，如 土豆',
  `category`   VARCHAR(32)     NOT NULL DEFAULT '其他' COMMENT '分类：蔬菜/肉禽/水产/蛋奶/主食/调味/干货/水果',
  `unit`       VARCHAR(16)     NOT NULL DEFAULT '克' COMMENT '默认计量单位',
  `emoji`      VARCHAR(16)     NOT NULL DEFAULT '🥬' COMMENT '展示 emoji',
  `is_staple`  TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '是否家中常备：1常备(采购清单可折叠) 0需采购',
  `is_active`  TINYINT(1)      NOT NULL DEFAULT 1,
  `created_at` DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_ingredient_name_unit` (`name`, `unit`),
  KEY `idx_ingredient_category` (`category`),
  KEY `idx_ingredient_staple` (`is_staple`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='食材字典';

-- ---------------------------------------------------------------------
-- 2.5 菜谱-食材关联（含用量，支撑采购清单按份数换算）
-- ---------------------------------------------------------------------
CREATE TABLE `recipe_ingredient` (
  `id`            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `recipe_id`     BIGINT UNSIGNED NOT NULL,
  `ingredient_id` BIGINT UNSIGNED NOT NULL,
  `quantity`      DECIMAL(10,2)   NOT NULL DEFAULT 0.00 COMMENT '基于 recipe.servings 的用量',
  `unit`          VARCHAR(16)     NOT NULL DEFAULT '克'  COMMENT '单位（可覆盖食材默认单位）',
  `group_name`    VARCHAR(32)     NOT NULL DEFAULT '主料' COMMENT '分组：主料/辅料/调味料',
  `is_optional`   TINYINT(1)      NOT NULL DEFAULT 0    COMMENT '可选配料：1是(不进采购清单)',
  `sort_order`    INT             NOT NULL DEFAULT 0,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_recipe_ingredient` (`recipe_id`, `ingredient_id`, `unit`),
  KEY `idx_ri_ingredient` (`ingredient_id`),
  CONSTRAINT `fk_ri_recipe` FOREIGN KEY (`recipe_id`) REFERENCES `recipe`(`id`)
    ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_ri_ingredient` FOREIGN KEY (`ingredient_id`) REFERENCES `ingredient`(`id`)
    ON DELETE RESTRICT ON UPDATE CASCADE,
  CONSTRAINT `chk_ri_quantity` CHECK (`quantity` >= 0 AND `quantity` <= 100000),
  CONSTRAINT `chk_ri_group` CHECK (`group_name` IN ('主料','辅料','调味料','其他'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='菜谱食材用量';

-- ---------------------------------------------------------------------
-- 2.6 标签
-- ---------------------------------------------------------------------
CREATE TABLE `tag` (
  `id`         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `name`       VARCHAR(32)  NOT NULL COMMENT '标签名，如 下饭/低卡/宝宝爱吃/快手菜',
  `color`      CHAR(7)      NOT NULL DEFAULT '#FF7043',
  `sort_order` INT          NOT NULL DEFAULT 0,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_tag_name` (`name`),
  CONSTRAINT `chk_tag_color` CHECK (`color` REGEXP '^#[0-9A-Fa-f]{6}$')
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='菜谱标签';

CREATE TABLE `recipe_tag` (
  `recipe_id` BIGINT UNSIGNED NOT NULL,
  `tag_id`    BIGINT UNSIGNED NOT NULL,
  PRIMARY KEY (`recipe_id`, `tag_id`),
  KEY `idx_rt_tag` (`tag_id`),
  CONSTRAINT `fk_rt_recipe` FOREIGN KEY (`recipe_id`) REFERENCES `recipe`(`id`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_rt_tag`    FOREIGN KEY (`tag_id`)    REFERENCES `tag`(`id`)    ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='菜谱-标签关联';

-- ---------------------------------------------------------------------
-- 2.7 成员收藏
-- ---------------------------------------------------------------------
CREATE TABLE `member_favorite` (
  `member_id`  BIGINT UNSIGNED NOT NULL,
  `recipe_id`  BIGINT UNSIGNED NOT NULL,
  `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`member_id`, `recipe_id`),
  KEY `idx_mf_recipe` (`recipe_id`),
  CONSTRAINT `fk_mf_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_mf_recipe` FOREIGN KEY (`recipe_id`) REFERENCES `recipe`(`id`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='成员收藏菜谱';

-- ---------------------------------------------------------------------
-- 2.8 点餐订单（一餐一张单）
-- ---------------------------------------------------------------------
CREATE TABLE `meal_order` (
  `id`          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `order_no`    VARCHAR(32)     NOT NULL COMMENT '订单号，如 20260930-D-001',
  `member_id`   BIGINT UNSIGNED NOT NULL COMMENT '下单人',
  `meal_date`   DATE            NOT NULL COMMENT '用餐日期',
  `meal_type`   ENUM('breakfast','lunch','dinner','snack') NOT NULL DEFAULT 'dinner' COMMENT '餐次',
  `status`      ENUM('draft','confirmed','cooking','served','cancelled') NOT NULL DEFAULT 'draft' COMMENT '订单状态',
  `diners`      TINYINT UNSIGNED NOT NULL DEFAULT 2 COMMENT '就餐人数',
  `note`        VARCHAR(512)    NULL COMMENT '整单备注',
  `served_at`   DATETIME        NULL COMMENT '开饭时间',
  `created_at`  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_order_no` (`order_no`),
  KEY `idx_order_date_type` (`meal_date`, `meal_type`),
  KEY `idx_order_member` (`member_id`),
  KEY `idx_order_status` (`status`),
  CONSTRAINT `fk_order_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)
    ON DELETE RESTRICT ON UPDATE CASCADE,
  CONSTRAINT `chk_order_meal_type` CHECK (`meal_type` IN ('breakfast','lunch','dinner','snack')),
  CONSTRAINT `chk_order_status`    CHECK (`status` IN ('draft','confirmed','cooking','served','cancelled')),
  CONSTRAINT `chk_order_diners`    CHECK (`diners` BETWEEN 1 AND 30)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='点餐订单';

-- ---------------------------------------------------------------------
-- 2.9 订单明细（点了哪些菜、各点几份、谁点的、吃完评分）
--     ★ recipe_snapshot 保存下单时的菜名，菜谱改名/删除后历史订单仍可追溯
-- ---------------------------------------------------------------------
CREATE TABLE `order_item` (
  `id`              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `order_id`        BIGINT UNSIGNED NOT NULL,
  `recipe_id`       BIGINT UNSIGNED NULL COMMENT '关联菜谱（菜谱被删后置空）',
  `recipe_snapshot` VARCHAR(64)     NOT NULL COMMENT '下单时菜名快照',
  `portions`        TINYINT UNSIGNED NOT NULL DEFAULT 1 COMMENT '点了几份',
  `member_id`       BIGINT UNSIGNED NULL COMMENT '这道菜是谁点的',
  `rating`          TINYINT UNSIGNED NULL COMMENT '餐后评分 1-5，NULL=未评',
  `item_status`     ENUM('pending','cooking','done','skipped') NOT NULL DEFAULT 'pending' COMMENT '出菜状态',
  `note`            VARCHAR(255)    NULL COMMENT '单项备注，如 不要辣',
  `sort_order`      INT             NOT NULL DEFAULT 0,
  `created_at`      DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_order_recipe_member` (`order_id`, `recipe_id`, `member_id`),
  KEY `idx_oi_recipe` (`recipe_id`),
  KEY `idx_oi_member` (`member_id`),
  KEY `idx_oi_rating` (`rating`),
  CONSTRAINT `fk_oi_order`  FOREIGN KEY (`order_id`)  REFERENCES `meal_order`(`id`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_oi_recipe` FOREIGN KEY (`recipe_id`) REFERENCES `recipe`(`id`)    ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT `fk_oi_member` FOREIGN KEY (`member_id`) REFERENCES `member`(`id`)    ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT `chk_oi_portions` CHECK (`portions` BETWEEN 1 AND 20),
  CONSTRAINT `chk_oi_rating`   CHECK (`rating` IS NULL OR `rating` BETWEEN 1 AND 5),
  CONSTRAINT `chk_oi_status`   CHECK (`item_status` IN ('pending','cooking','done','skipped'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='订单明细';

-- ---------------------------------------------------------------------
-- 2.10 采购清单（由订单聚合生成，可勾选已买）
-- ---------------------------------------------------------------------
CREATE TABLE `shopping_item` (
  `id`             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  `order_id`       BIGINT UNSIGNED NOT NULL,
  `ingredient_id`  BIGINT UNSIGNED NULL,
  `name_snapshot`  VARCHAR(64)     NOT NULL COMMENT '食材名快照',
  `emoji`          VARCHAR(16)     NOT NULL DEFAULT '🥬',
  `category`       VARCHAR(32)     NOT NULL DEFAULT '其他',
  `total_quantity` DECIMAL(10,2)   NOT NULL DEFAULT 0.00 COMMENT '按份数换算后的合计用量',
  `unit`           VARCHAR(16)     NOT NULL DEFAULT '克',
  `is_staple`      TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '家中常备',
  `is_purchased`   TINYINT(1)      NOT NULL DEFAULT 0 COMMENT '是否已购买',
  `recipe_names`   VARCHAR(512)    NULL COMMENT '来源菜品，逗号分隔',
  `created_at`     DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at`     DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_shop_order_ing_unit` (`order_id`, `ingredient_id`, `unit`),
  KEY `idx_shop_purchased` (`is_purchased`),
  CONSTRAINT `fk_shop_order` FOREIGN KEY (`order_id`) REFERENCES `meal_order`(`id`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_shop_ingredient` FOREIGN KEY (`ingredient_id`) REFERENCES `ingredient`(`id`)
    ON DELETE SET NULL ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='订单采购清单';

-- =====================================================================
--  3. 视图
-- =====================================================================

-- ---------------------------------------------------------------------
-- 3.1 菜谱完整视图：菜谱 + 标签/食材/步骤数量
-- ---------------------------------------------------------------------
CREATE OR REPLACE VIEW `v_recipe_full` AS
SELECT
  r.*,
  (SELECT COUNT(*) FROM `recipe_step`       s  WHERE s.recipe_id  = r.id) AS step_count,
  (SELECT COUNT(*) FROM `recipe_ingredient` ri WHERE ri.recipe_id = r.id) AS ingredient_count,
  (SELECT COUNT(*) FROM `member_favorite`   f  WHERE f.recipe_id  = r.id) AS favorite_count,
  (SELECT GROUP_CONCAT(t.`name` ORDER BY t.sort_order SEPARATOR ',')
     FROM `recipe_tag` rt JOIN `tag` t ON t.id = rt.tag_id
    WHERE rt.recipe_id = r.id) AS tag_names
FROM `recipe` r;

-- ---------------------------------------------------------------------
-- 3.2 订单明细视图（含菜品当前信息 + 点餐人）
-- ---------------------------------------------------------------------
CREATE OR REPLACE VIEW `v_order_detail` AS
SELECT
  o.id            AS order_id,
  o.order_no,
  o.meal_date,
  o.meal_type,
  o.status        AS order_status,
  o.diners,
  o.note          AS order_note,
  oi.id           AS item_id,
  oi.recipe_id,
  oi.recipe_snapshot,
  oi.portions,
  oi.rating,
  oi.item_status,
  r.`name`        AS recipe_name,
  r.cover_emoji,
  r.cover_gradient,
  r.category,
  r.difficulty,
  r.total_minutes,
  m.nickname      AS ordered_by,
  m.avatar_emoji  AS ordered_by_avatar
FROM `meal_order` o
JOIN `order_item` oi ON oi.order_id = o.id
LEFT JOIN `recipe`  r  ON r.id = oi.recipe_id
LEFT JOIN `member`  m  ON m.id = oi.member_id;

-- ---------------------------------------------------------------------
-- 3.3 ★核心：采购清单预览
--     按 (食材, 单位) 聚合，用量 = Σ(菜谱用量 × 点单份数 / 菜谱默认份数)
--     单位不同的同名食材分开统计，避免「土豆 300克 + 2个」被错误相加
-- ---------------------------------------------------------------------
CREATE OR REPLACE VIEW `v_order_shopping_preview` AS
SELECT
  oi.order_id,
  i.id                     AS ingredient_id,
  i.`name`                 AS ingredient_name,
  i.emoji,
  i.category,
  COALESCE(NULLIF(ri.unit, ''), i.unit) AS unit,
  ROUND(SUM(ri.quantity * (oi.portions / NULLIF(r.servings, 0))), 2) AS total_quantity,
  COUNT(DISTINCT r.id)     AS recipe_count,
  GROUP_CONCAT(DISTINCT oi.recipe_snapshot ORDER BY oi.recipe_snapshot SEPARATOR '、') AS recipe_names,
  MAX(i.is_staple)         AS is_staple
FROM `order_item` oi
JOIN `meal_order`        mo ON mo.id = oi.order_id
JOIN `recipe`            r  ON r.id  = oi.recipe_id
JOIN `recipe_ingredient` ri ON ri.recipe_id = r.id
JOIN `ingredient`        i  ON i.id  = ri.ingredient_id
WHERE mo.status <> 'cancelled'
  AND oi.item_status <> 'skipped'
  AND ri.is_optional = 0
GROUP BY
  oi.order_id, i.id, i.`name`, i.emoji, i.category,
  COALESCE(NULLIF(ri.unit, ''), i.unit);

-- ---------------------------------------------------------------------
-- 3.4 成员口味统计：谁爱吃什么分类、平均打几分
-- ---------------------------------------------------------------------
CREATE OR REPLACE VIEW `v_member_taste_stat` AS
SELECT
  m.id            AS member_id,
  m.nickname,
  m.avatar_emoji,
  m.theme_color,
  r.category,
  COUNT(*)                       AS order_times,
  SUM(oi.portions)               AS total_portions,
  ROUND(AVG(oi.rating), 2)       AS avg_rating,
  COUNT(oi.rating)               AS rated_times,
  MAX(o.meal_date)               AS last_ordered_at
FROM `order_item` oi
JOIN `meal_order` o  ON o.id = oi.order_id AND o.status <> 'cancelled'
JOIN `member`     m  ON m.id = oi.member_id
JOIN `recipe`     r  ON r.id = oi.recipe_id
GROUP BY m.id, m.nickname, m.avatar_emoji, m.theme_color, r.category;

-- ---------------------------------------------------------------------
-- 3.5 菜品受欢迎度排行
-- ---------------------------------------------------------------------
CREATE OR REPLACE VIEW `v_category_stat` AS
SELECT
  r.id,
  r.`name`,
  r.category,
  r.cover_emoji,
  r.cover_gradient,
  r.avg_rating,
  r.cook_count,
  COUNT(CASE WHEN o.status <> 'cancelled' THEN 1 END)          AS real_order_count,
  ROUND(AVG(CASE WHEN o.status <> 'cancelled' THEN oi.rating END), 2) AS live_avg_rating,
  SUM(CASE WHEN o.status <> 'cancelled' THEN oi.portions ELSE 0 END)  AS total_portions,
  ROW_NUMBER() OVER (
    PARTITION BY r.category
    ORDER BY COUNT(CASE WHEN o.status <> 'cancelled' THEN 1 END) DESC,
             AVG(CASE WHEN o.status <> 'cancelled' THEN oi.rating END) DESC
  ) AS rank_in_category
FROM `recipe` r
LEFT JOIN `order_item` oi ON oi.recipe_id = r.id
LEFT JOIN `meal_order`  o ON o.id = oi.order_id
GROUP BY r.id, r.`name`, r.category, r.cover_emoji, r.cover_gradient, r.avg_rating, r.cook_count;

-- =====================================================================
--  4. 存储过程
-- =====================================================================

DELIMITER $$

-- ---------------------------------------------------------------------
-- 4.1 生成/刷新某订单的采购清单（幂等：先删后插）
--     注意：本过程不含事务控制，请在应用层的事务中调用。
-- ---------------------------------------------------------------------
DROP PROCEDURE IF EXISTS `sp_generate_shopping_list`$$
CREATE PROCEDURE `sp_generate_shopping_list`(IN p_order_id BIGINT UNSIGNED)
BEGIN
  DECLARE v_exists INT DEFAULT 0;

  SELECT COUNT(*) INTO v_exists FROM `meal_order` WHERE id = p_order_id;
  IF v_exists = 0 THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '订单不存在';
  END IF;

  DELETE FROM `shopping_item` WHERE `order_id` = p_order_id;

  INSERT INTO `shopping_item`
    (`order_id`, `ingredient_id`, `name_snapshot`, `emoji`, `category`,
     `total_quantity`, `unit`, `is_staple`, `recipe_names`)
  SELECT
    p.order_id, p.ingredient_id, p.ingredient_name, p.emoji, p.category,
    p.total_quantity, p.unit, p.is_staple, p.recipe_names
  FROM `v_order_shopping_preview` p
  WHERE p.order_id = p_order_id
  ORDER BY p.is_staple ASC, p.category ASC, p.ingredient_name ASC;
END$$

-- ---------------------------------------------------------------------
-- 4.2 确认订单：置状态 + 生成采购清单 + 累加菜谱点单次数
--     单事务完成，保证一致性
-- ---------------------------------------------------------------------
DROP PROCEDURE IF EXISTS `sp_confirm_order`$$
CREATE PROCEDURE `sp_confirm_order`(IN p_order_id BIGINT UNSIGNED)
BEGIN
  DECLARE v_status VARCHAR(16);

  SELECT `status` INTO v_status FROM `meal_order` WHERE id = p_order_id FOR UPDATE;
  IF v_status IS NULL THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '订单不存在';
  END IF;
  IF v_status = 'cancelled' THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '已取消的订单无法确认';
  END IF;

  UPDATE `meal_order` SET `status` = 'confirmed' WHERE id = p_order_id;

  -- 重算本单涉及菜谱的冗余统计（幂等）
  --
  -- 这里刻意不用 `cook_count = cook_count + 1` 的增量写法：
  -- 增量方案要求每条改变统计的路径都记得做反向补偿，而取消订单、
  -- 删除订单、删除单项菜品都容易漏掉回退，导致 cook_count 只增不减、
  -- avg_rating 被已删订单的评分污染，且偏差会长期累积。
  -- 重算以订单明细为唯一真相来源，无论调用多少次结果都一致。
  UPDATE `recipe` r
     SET r.cook_count = COALESCE((
           SELECT CAST(SUM(oi.portions) AS SIGNED)
             FROM `order_item` oi JOIN `meal_order` o ON o.id = oi.order_id
            WHERE oi.recipe_id = r.id
              AND o.status NOT IN ('cancelled','draft')
              AND oi.item_status <> 'skipped'
         ), 0),
         r.avg_rating = COALESCE((
           SELECT ROUND(AVG(oi.rating),2)
             FROM `order_item` oi JOIN `meal_order` o ON o.id = oi.order_id
            WHERE oi.recipe_id = r.id AND oi.rating IS NOT NULL
              AND o.status NOT IN ('cancelled','draft')
         ), 0)
   WHERE r.id IN (SELECT DISTINCT oi2.recipe_id FROM `order_item` oi2
                   WHERE oi2.order_id = p_order_id AND oi2.recipe_id IS NOT NULL);

  CALL `sp_generate_shopping_list`(p_order_id);
END$$

DELIMITER ;

-- =====================================================================
--  5. 事件（可选）：每天凌晨重算菜谱平均评分，保持冗余字段准确
--     需要 event_scheduler=ON，默认注释掉，按需开启
-- =====================================================================
-- SET GLOBAL event_scheduler = ON;
-- DELIMITER $$
-- CREATE EVENT IF NOT EXISTS `ev_refresh_recipe_rating`
-- ON SCHEDULE EVERY 1 DAY STARTS (CURRENT_DATE + INTERVAL 1 DAY + INTERVAL 3 HOUR)
-- DO BEGIN
--   UPDATE `recipe` r SET r.avg_rating = COALESCE((
--     SELECT ROUND(AVG(oi.rating),2) FROM `order_item` oi
--      WHERE oi.recipe_id = r.id AND oi.rating IS NOT NULL
--   ), 0);
-- END$$
-- DELIMITER ;

SET FOREIGN_KEY_CHECKS = 1;

-- =====================================================================
--  架构创建完成。执行 02_seed_data.sql 导入示例数据。
-- =====================================================================
