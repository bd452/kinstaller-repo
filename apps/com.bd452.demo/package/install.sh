#!/bin/sh

set -e

chmod +x app.sh

if [ -f /lib/ld-linux-armhf.so.3 ]; then
    PLAT=kindlehf
else
    PLAT=kindlepw2
fi

cat > /mnt/us/documents/com.bd452.demo.sh << EOF
#!/bin/sh
exec /var/local/kmc/${PLAT}/bin/kpm launch com.bd452.demo
EOF
chmod +x /mnt/us/documents/com.bd452.demo.sh

echo "Demo installed. Open com.bd452.demo.sh from Documents to launch."
