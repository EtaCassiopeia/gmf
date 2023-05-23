To enable IPv6 for your Docker daemon, you need to configure Docker daemon settings. Here are the general steps to do this:

1. Open or create a Docker daemon configuration file. This is typically located at `/etc/docker/daemon.json`. If it doesn't exist, you can create it.

2. In the configuration file, you need to set `ipv6` to `true` and specify a `fixed-cidr-v6` (a default IPv6 subnet). Here is an example of what the file should look like:

```json
{
  "ipv6": true,
  "fixed-cidr-v6": "2001:db8:1::/64"
}
```

3. Save and close the file.

4. Restart the Docker daemon for the changes to take effect. On a Unix-based system, you can typically do this by running `systemctl restart docker` as root (or with `sudo`).

Please note that the `fixed-cidr-v6` value (`2001:db8:1::/64` in the example) is an IPv6 subnet used for the IPv6 address of the bridge named `docker0`, which Docker creates by default. It's also used for assigning IPv6 addresses to your containers. If your system is set up differently, you may need to use a different subnet.

Finally, please be aware that making these changes could have implications for your network setup, and you should ensure that you understand these before proceeding. Make sure to back up any important data before making changes to your system configuration.