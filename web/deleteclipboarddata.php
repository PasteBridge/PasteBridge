<?php
    include_once("database-login.php");

    $copyroom_name = $_POST["copyroom-name"];

    $sql_request = "DELETE FROM clipboarddata WHERE clipboarddata_copyroom = '" . sqlite_escape($conn, $copyroom_name) . "'";
                
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