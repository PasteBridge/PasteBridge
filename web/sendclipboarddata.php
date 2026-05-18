<?php
    include_once("database-login.php");

    $textToSave = $_POST["text_to_send"];
    $copyroom_name = $_POST["copyroom-name"];

    $max_id_result = sqlite_query($conn, "SELECT MAX(clipboarddata_id) as max_id FROM clipboarddata");
    $max_id_row = sqlite_fetch_array($max_id_result);
    $clipboarddata_id = ($max_id_row['max_id'] ?? 0) + 1;
    
    $count_result = sqlite_query($conn, "SELECT COUNT(*) as cnt FROM clipboarddata WHERE clipboarddata_copyroom = '" . sqlite_escape($conn, $copyroom_name) . "'");
    $count_row = sqlite_fetch_array($count_result);
    $copyroom_number = ($count_row['cnt'] ?? 0) + 1;
    $current_time = time();

    $sql_request = "INSERT INTO clipboarddata (clipboarddata_id, clipboarddata_createtime, clipboarddata_content, clipboarddata_ip, clipboarddata_copyroom) VALUES ({$clipboarddata_id}, {$current_time}, '" . sqlite_escape($conn, $textToSave) . "', 1, '" . sqlite_escape($conn, $copyroom_name) . "')";
    sqlite_query($conn, "UPDATE copyroom SET copyroom_number = {$copyroom_number} WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'");
        
    if (sqlite_query($conn, $sql_request)){ 
        $sql_status = true;
    }else{
        $sql_status = false;
    }
    
    $retarr[] = array(
        'status1' => $sql_status,
        'status' => $sql_request,
    ) ;  
            
    echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
?>