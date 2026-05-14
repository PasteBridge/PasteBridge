-- phpMyAdmin SQL Dump
-- version 4.8.5
-- https://www.phpmyadmin.net/
--
-- 主机： localhost
-- 生成日期： 2022-08-26 02:35:05
-- 服务器版本： 5.7.26
-- PHP 版本： 7.3.4

SET SQL_MODE = "NO_AUTO_VALUE_ON_ZERO";
SET AUTOCOMMIT = 0;
START TRANSACTION;
SET time_zone = "+00:00";


/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!40101 SET NAMES utf8mb4 */;

--
-- 数据库： `purecopy`
--

-- --------------------------------------------------------

--
-- 表的结构 `clipboarddata`
--

CREATE TABLE `clipboarddata` (
  `clipboarddata_id` int(11) NOT NULL,
  `clipboarddata_createtime` int(11) NOT NULL,
  `clipboarddata_content` mediumtext CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_520_ci NOT NULL,
  `clipboarddata_ip` int(11) NOT NULL,
  `clipboarddata_copyroom` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_520_ci NOT NULL
) ENGINE=MyISAM DEFAULT CHARSET=utf8;

-- --------------------------------------------------------

--
-- 表的结构 `copyroom`
--

CREATE TABLE `copyroom` (
  `copyroom_id` int(11) NOT NULL,
  `copyroom_name` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_520_ci NOT NULL,
  `copyroom_create_time` int(11) NOT NULL,
  `copyroom_lastseen_time` int(11) NOT NULL,
  `copyroom_password` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_520_ci NOT NULL,
  `admin` int(1) NOT NULL DEFAULT '0',
  `copyroom_number` int(11) NOT NULL
) ENGINE=MyISAM DEFAULT CHARSET=utf8;

--
-- 转储表的索引
--

--
-- 表的索引 `clipboarddata`
--
ALTER TABLE `clipboarddata`
  ADD PRIMARY KEY (`clipboarddata_id`);

--
-- 表的索引 `copyroom`
--
ALTER TABLE `copyroom`
  ADD PRIMARY KEY (`copyroom_id`);
COMMIT;

/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
