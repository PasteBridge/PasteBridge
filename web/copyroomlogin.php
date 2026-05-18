<?php
    include_once("database-login.php");

    $copyroom_name = $_POST["copyroom-name"];
    $copyroom_password = $_POST["copyroom-password"];
    $applicability_check = $_POST["applicability-check"];
    
    if(empty($copyroom_password)){
        $copyroom_password = null;
        $password_hash = null;
    }else{
        $password_hash = password_hash($copyroom_password,PASSWORD_DEFAULT);    
    };
    
    $max_id_result = sqlite_query($conn, "SELECT MAX(copyroom_id) as max_id FROM copyroom");
    $max_id_row = sqlite_fetch_array($max_id_result);
    $copyroom_id = ($max_id_row['max_id'] ?? 0) + 1;

    setcookie("copyroom_name_latest", "$copyroom_name", time()+3600);
    
    $admin_result = sqlite_query($conn, "SELECT admin FROM copyroom WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'");
    $admin_row = sqlite_fetch_array($admin_result);
    $admin = $admin_row ? $admin_row['admin'] : 0;

    $password_result = sqlite_query($conn, "SELECT copyroom_password FROM copyroom WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'");
    $password_row = sqlite_fetch_array($password_result);
    $copyroom_password_db = $password_row ? $password_row['copyroom_password'] : null;
    
    $name_result = sqlite_query($conn, "SELECT copyroom_name FROM copyroom WHERE copyroom_name = '" . sqlite_escape($conn, $copyroom_name) . "'");
    $name_row = sqlite_fetch_array($name_result);
    $copyroom_db = $name_row ? $name_row['copyroom_name'] : null;
    
    if($copyroom_db == null){
        $copyroom_exists = false;
        $password_exists = false;
            
        if($applicability_check){
            // just check, do nothing
        }else{
            $sql_request = "INSERT INTO copyroom (copyroom_id, copyroom_name, copyroom_create_time, copyroom_lastseen_time, copyroom_password, admin, copyroom_number) VALUES ({$copyroom_id},'" . sqlite_escape($conn, $copyroom_name) . "', " . time() . ", " . time() . ",'" . ($password_hash ?? '') . "', 0, 0)";
            
            if (sqlite_query($conn, $sql_request)){ 
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
    } else {
        $copyroom_exists = true;
        if($copyroom_password_db == null || $copyroom_password_db === ''){
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
        }else{
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