hex_str = "00:c6:0f:18:37:11:c0:93:81:98:1f:f9:8b:92:6b:07:53:05:58:b1:e5:d2:f3:73:0e:14:41:6d:79:53:fb:c6:6d:7d:69:d8:e1:73:28:8e:1a:c7:87:06:7c:8c:de:4e:e9:e5:5f:4c:23:06:24:50:f1:78:c4:b1:ef:27:7a:ee:df:15:c6:01:c4:ff:07:5e:6d:5f:4d:47:2f:a4:fd:a9:7c:67:a2:f7:0d:30:79:c2:da:51:3f:cf:a1:9f:0b:fc:74:a1:bb:a3:b1:0e:ea:52:2f:77:9d:c1:d0:c0:a7:74:9d:53:d4:69:4a:c0:ac:11:86:c0:e5:ec:bf:b1:44:c6:2d"
hex_str_no_colon = hex_str.replace(":", "")
decimal_value = int(hex_str_no_colon, 16)
print(decimal_value)
