<?php
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $textToSave =$_POST["text_to_send"];
    $copyroom_name = $_POST["copyroom-name"];

    $max_id = mysqli_fetch_array($conn -> query("SELECT MAX(clipboarddata_id) FROM clipboarddata"));
    $clipboarddata_id = $max_id[0] + 1;
    
    $sql_request = "DELETE FROM `clipboarddata` WHERE `clipboarddata_copyroom` = '{$copyroom_name}'";
                
    if ($conn -> query($sql_request) === TRUE){ 
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