<?php
    include_once("database-login.php");

    $copyroom_name = $_POST["copyroom-name"];
    $copyroom_password = $_POST["copyroom-password"];
    
    if(empty($copyroom_password)){
        $copyroom_password = null;
        $password_hash = null;
    }else{
        $password_hash = password_hash($copyroom_password,PASSWORD_DEFAULT);    
    };
    
    $sql_request = "UPDATE copyroom SET copyroom_password = '" . ($password_hash ?? '') . "' WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'";
                
    if (sqlite_query($conn, $sql_request)){ 
        $sql_status = true;
    }else{
        $sql_status = false;
    }

    $retarr[] = array(
        'copyroom_name' => $copyroom_name,
        '$sql_status' => $sql_status,
        'password_verify' => true,
    ) ;  
            
    echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
?>