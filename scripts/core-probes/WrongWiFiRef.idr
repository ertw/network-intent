module WrongWiFiRef
import NetDSL.AAA
%default total
bad : SecretRef PasswordHash -> SecretRef WiFiCredential
bad ref = ref
