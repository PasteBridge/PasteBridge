<?php
    include_once("database-password.php");
    include_once("database-login.php");
    
    mysqli_select_db($conn , "purecopy");

    $password = $_POST["password"];
    $password_hash = password_hash($password,PASSWORD_DEFAULT);
    
    if(password_verify($password,$password_hash)){
        echo $password_hash;
        echo "true";
    };

?>