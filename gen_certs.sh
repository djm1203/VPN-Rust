#!/bin/bash

echo "🔐 Generating TLS certificates for VPN testing..."

# Create certs directory
mkdir -p certs

# Generate private key
openssl genrsa -out certs/server.key 2048
echo "✅ Generated private key"

# Generate certificate signing request
openssl req -new -key certs/server.key -out certs/server.csr -subj "/C=US/ST=Test/L=Test/O=VPN-Test/CN=localhost"
echo "✅ Generated certificate signing request"

# Generate self-signed certificate
openssl x509 -req -days 365 -in certs/server.csr -signkey certs/server.key -out certs/server.crt
echo "✅ Generated self-signed certificate"

# Clean up CSR file
rm certs/server.csr

# Show certificate info
echo
echo "📋 Certificate information:"
openssl x509 -in certs/server.crt -text -noout | grep -E "(Subject:|Not Before:|Not After :|DNS:|IP Address:)"

echo
echo "✅ Certificates ready!"
echo "   Private key: certs/server.key"
echo "   Certificate: certs/server.crt"
