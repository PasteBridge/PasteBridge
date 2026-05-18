<?php
$db_path = __DIR__ . "/data/purecopy.db";
$conn = new PDO("sqlite:" . $db_path);
$conn->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

$sql = "
CREATE TABLE IF NOT EXISTS `clipboarddata` (
  `clipboarddata_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `clipboarddata_createtime` INTEGER NOT NULL,
  `clipboarddata_content` TEXT NOT NULL,
  `clipboarddata_ip` INTEGER NOT NULL,
  `clipboarddata_copyroom` TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS `copyroom` (
  `copyroom_id` INTEGER PRIMARY KEY AUTOINCREMENT,
  `copyroom_name` TEXT NOT NULL,
  `copyroom_create_time` INTEGER NOT NULL,
  `copyroom_lastseen_time` INTEGER NOT NULL,
  `copyroom_password` TEXT NOT NULL,
  `admin` INTEGER NOT NULL DEFAULT 0,
  `copyroom_number` INTEGER NOT NULL
);
";

$conn->exec($sql);
echo "Database created at: " . $db_path;
?>
