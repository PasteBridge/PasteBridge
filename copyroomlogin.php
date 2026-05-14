<?php
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $copyroom_name = $_POST["copyroom-name"];
    $copyroom_password = $_POST["copyroom-password"];
    $applicability_check = $_POST["applicability-check"];
    
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

    setcookie("copyroom_name_latest", "$copyroom_name", time()+3600);
    
    $admin = mysqli_fetch_array($conn -> query("SELECT admin FROM copyroom WHERE `copyroom_name` LIKE '%{$copyroom_name}%'"))[0];

    //首次使用copyroom时创建
    $copyroom_password_db = mysqli_fetch_array($conn -> query("SELECT copyroom_password FROM copyroom WHERE `copyroom_name` LIKE '%{$copyroom_name}%'"))[0];
    $copyroom_db = mysqli_fetch_array($conn -> query("SELECT copyroom_name FROM copyroom WHERE `copyroom_name` LIKE '%{$copyroom_name}%'"))[0];
    if($copyroom_db == null){
        $copyroom_exists = false;
            $password_exists = false;
            
            if($applicability_check){

            }else{
                $sql_request = "INSERT INTO copyroom (`copyroom_id`,`copyroom_name`, `copyroom_create_time`, `copyroom_lastseen_time`, `copyroom_password`) VALUES ({$copyroom_id},'{$copyroom_name}', '1', '1','{$password_hash}')";
                
                if ($conn -> query($sql_request) === TRUE){ 
                    $sql_status = true;
                }else{
                    $sql_status = false;
                }

            }

            
            $retarr[] = array(
                'copyroom_name' => $copyroom_name,
                'password_exists' => $password_exists,
                'copyroom_exists' => $copyroom_exists,
                '$sql_status' => $sql_status,
                'password_verify' => true,
            ) ;  
            
            echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
    }
    
    else{
        $copyroom_exists = true;
        if($copyroom_password_db == null){
            $password_verified = true;
            $password_exists = false;
            $no_password = true;
            $retarr[] = array(
                'copyroom_name' => $copyroom_name,
                'password_exists' => $password_exists,
                'copyroom_exists' => $copyroom_exists,
                '$sql_status' => $sql_status,
                'password_verify' => $password_verified,
                'password_change_require' => true,
                'no_password' => $no_password,
                'admin' => $admin
            ) ;   
            
            echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
        }else{//第二次使用copyroom时要求输入密码
            $password_exists = true;
            if(password_verify($copyroom_password,$copyroom_password_db)){
                $password_verified = true;
            }else{
                $password_verified = false;
            };

            $retarr[] = array(
                'copyroom_name' => $copyroom_name,
                'password_exists' => $password_exists,
                'copyroom_exists' => $copyroom_exists,
                '$sql_status' => $sql_status,
                'password_verify' => $password_verified,
            ) ;  
            
            echo json_encode($retarr,JSON_UNESCAPED_UNICODE);
        }
        
    }
    

?>