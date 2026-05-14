<?php
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $user_token = $_POST["token"];
    $copyroom_name = $_POST["copyroom-name"];
    
    $content = mysqli_fetch_all($conn -> query("SELECT `clipboarddata_id`, `clipboarddata_createtime`, `clipboarddata_content`, `clipboarddata_ip`, `clipboarddata_copyroom` FROM clipboarddata WHERE `clipboarddata_copyroom` = '{$copyroom_name}' ORDER BY `clipboarddata`.`clipboarddata_createtime` DESC"),MYSQLI_ASSOC);
    
    $copyroom_number = mysqli_fetch_all($conn -> query("SELECT `copyroom_number` FROM copyroom WHERE `copyroom_name` = '{$copyroom_name}'"));

    $retarr[] = array(
        'clipboard_data' => $content,
        'copyroom_number' => $copyroom_number,
        'request_name' => $copyroom_name,
    ) ;  
            
    echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
?>