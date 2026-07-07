#!/bin/sh

set -e

chmod +x app.sh

cat > /mnt/us/documents/com.bd452.demo.sh << 'EOF'
#!/bin/sh
exec /var/local/kmc/bin/kpm launch com.bd452.demo
EOF
chmod +x /mnt/us/documents/com.bd452.demo.sh

echo "Demo installed. Open com.bd452.demo.sh from Documents to launch."
