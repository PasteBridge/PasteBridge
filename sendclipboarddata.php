<?php
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $textToSave =$_POST["text_to_send"];
    $copyroom_name = $_POST["copyroom-name"];

    $max_id = mysqli_fetch_array($conn -> query("SELECT MAX(clipboarddata_id) FROM clipboarddata"));
    $clipboarddata_id = $max_id[0] + 1;
    

    $copyroom_number = mysqli_fetch_array($conn -> query("SELECT COUNT(clipboarddata_copyroom) FROM clipboarddata WHERE clipboarddata_copyroom = '{$copyroom_name}'"));
    $copyroom_number = $copyroom_number[0] + 1;
    $current_time = time();

    $copyroom_name = mysqli_real_escape_string($conn,$copyroom_name);
    $textToSave = mysqli_real_escape_string($conn,$textToSave);

    $sql_request = "INSERT INTO `clipboarddata` (`clipboarddata_id`, `clipboarddata_createtime`, `clipboarddata_content`, `clipboarddata_ip`, `clipboarddata_copyroom`) VALUES ({$clipboarddata_id},'{$current_time}', '{$textToSave}','1','{$copyroom_name}')";
    $conn -> query("UPDATE `copyroom` SET `copyroom_number` = '{$copyroom_number}' WHERE `copyroom`.`copyroom_name` = '{$copyroom_name}'");
        
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