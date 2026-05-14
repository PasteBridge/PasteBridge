<?php
    $db_servername="";
    $db_username="";
    $db_password="";
    $conn = new mysqli($db_servername , $db_username, $db_password);

if ($conn -> connect_error) {
    die("ERROR CONNECTING" . $conn -> connect_error);
}
?>