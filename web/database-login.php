<?php
    $db_path = __DIR__ . "/data/purecopy.db";
    
    // 创建数据目录
    if (!is_dir(__DIR__ . "/data")) {
        mkdir(__DIR__ . "/data", 0777, true);
    }
    
    try {
        $conn = new PDO("sqlite:" . $db_path);
        $conn->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $conn->exec("PRAGMA journal_mode=WAL");
    } catch (PDOException $e) {
        die("ERROR CONNECTING: " . $e->getMessage());
    }
    
    // SQLite辅助函数
    function sqlite_query($conn, $sql) {
        return $conn->query($sql);
    }
    
    function sqlite_fetch_array($result) {
        return $result->fetch(PDO::FETCH_ASSOC);
    }
    
    function sqlite_fetch_all($result) {
        return $result->fetchAll(PDO::FETCH_ASSOC);
    }
    
    function sqlite_escape($conn, $str) {
        return substr($conn->quote($str), 1, -1);
    }
    
    function sqlite_last_insert_id($conn) {
        return $conn->lastInsertId();
    }
?>