<?php
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $copyroom_name = $_POST["copyroom-name"];
    $copyroom_password = $_POST["copyroom-password"];
    
    if(empty($copyroom_password)){
        $copyroom_password = null;
        $password_hash = null;
    }else{
        //密码处理
        $password_hash = password_hash($copyroom_password,PASSWORD_DEFAULT);    
    };
    
    $max_id = mysqli_fetch_array($conn -> query("SELECT MAX(copyroom_id) FROM copyroom"));
    $copyroom_id = $max_id[0] + 1;
    
    //token
    // $token = md5(time() + $copyroom_name + $copyroom_password);

    //首次使用copyroom时创建

    $sql_request = "UPDATE `copyroom` SET `copyroom_password` = '{$password_hash}' WHERE `copyroom`.`copyroom_name` = '{$copyroom_name}';";
                
                if ($conn -> query($sql_request) === TRUE){ 
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