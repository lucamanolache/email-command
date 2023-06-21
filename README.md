Email Command
=============

This project arose from my need to get notifications when my code finished
running on my server while on vacation. The code will send an email notification
when the command it is running finishes.

To set it up, create a file called `config.toml`

``` toml
[email]
to_address = "<sendtome@address.com>"
smtp_username = "<sendfroma@address.com>"
smtp_password = "<password>"
smtp_server = "<smtp server>"
```
