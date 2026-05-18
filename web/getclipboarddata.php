<?php
    include_once("database-login.php");

    $user_token = $_POST["token"];
    $copyroom_name = $_POST["copyroom-name"];
    
    $content_result = sqlite_query($conn, "SELECT clipboarddata_id, clipboarddata_createtime, clipboarddata_content, clipboarddata_ip, clipboarddata_copyroom FROM clipboarddata WHERE clipboarddata_copyroom = '" . sqlite_escape($conn, $copyroom_name) . "' ORDER BY clipboarddata_createtime DESC");
    $content = sqlite_fetch_all($content_result);
    
    $number_result = sqlite_query($conn, "SELECT copyroom_number FROM copyroom WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'");
    $copyroom_number = sqlite_fetch_all($number_result);

    $retarr[] = array(
        'clipboard_data' => $content,
        'copyroom_number' => $copyroom_number,
        'request_name' => $copyroom_name,
    ) ;  
            
    echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
?>