package com.marekkon5.gdvpn;

import android.net.VpnService;
import android.util.Log;

import com.google.api.services.drive.Drive;

import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.Socket;
import java.net.URL;
import java.nio.ByteBuffer;
import java.nio.channels.Channels;
import java.nio.channels.FileChannel;
import java.nio.channels.ReadableByteChannel;
import java.nio.channels.SocketChannel;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class VPNConnection {

    private static final String TAG = "GDVPN";

    private Socket socket;
    private SocketChannel channel;
    private List<String> fileIds = new ArrayList<String>();
    private ExecutorService executor;
    private FileDescriptor vpnFileDescriptor;
    private Drive drive;

    VPNConnection(FileDescriptor vpnFileDescriptor, Drive drive) {
        this.vpnFileDescriptor = vpnFileDescriptor;
        this.executor = Executors.newFixedThreadPool(8);
        this.drive = drive;
    }

    /// Connect to a VPN server
    public void connect(String host, int port) throws Exception {
        socket = new Socket(host, port);
        channel = socket.getChannel();
        readFileIds();

        VPNReader reader = new VPNReader();
        VPNWriter writer = new VPNWriter();
        executor.submit(reader);
        executor.submit(writer);
    }

    /// Read exact size
    ByteBuffer readExact(ByteBuffer buffer) throws Exception {
        for (int read = 0; read < buffer.limit(); read += channel.read(buffer));
        buffer.flip();
        return buffer;
    }

    // Receive initial file list
    void readFileIds() throws Exception {
        ByteBuffer lenBuf = ByteBuffer.allocate(4);
        lenBuf = readExact(lenBuf);
        int len = lenBuf.getInt();
        // Receive initial file list
        ByteBuffer buf = ByteBuffer.allocate(len);
        buf = readExact(buf);
        String data = new String(Arrays.copyOfRange(buf.array(), 0, buf.limit()), "UTF-8");
        fileIds.addAll(Arrays.asList(data.split("\n")));
    }

    InputStream getDriveFile(String fileId) throws Exception {
        return drive.files().get(fileId).executeMediaAsInputStream();
    }

    private class VPNReader implements Runnable {

        @Override
        public void run() {
            FileChannel vpnInput = new FileInputStream(vpnFileDescriptor).getChannel();
            ByteBuffer buffer = ByteBuffer.allocate(1500);
            ByteBuffer lenBuffer = ByteBuffer.allocate(2);

            try {
                while (true) {
                    int read = vpnInput.read(buffer);
                    if (read > 0) {
//                        Log.d(TAG, "READ: " + Integer.toString(read));
                        buffer.limit(read);
                        buffer.flip();
                        lenBuffer.putShort((short)read);
                        lenBuffer.flip();
                        channel.write(lenBuffer);
                        channel.write(buffer);
                        buffer.clear();
                        lenBuffer.clear();
                    }
                }
            } catch (Exception e) {
                e.printStackTrace();
            }
        }
    }

    private class VPNWriter implements Runnable {

        @Override
        public void run() {
            FileChannel vpnOutput = new FileOutputStream(vpnFileDescriptor).getChannel();
            ByteBuffer lenBuffer = ByteBuffer.allocate(2);
            ByteBuffer buffer = ByteBuffer.allocate(1500);
            try {
                while (true) {
                    lenBuffer = readExact(lenBuffer);
                    int index = (int)lenBuffer.getShort();
                    lenBuffer.clear();


                    // Google drive
                    String fileId = fileIds.get(index);
                    Log.d(TAG, "File: " + fileId);


                    ReadableByteChannel in = Channels.newChannel(getDriveFile(fileId));
                    while (true) {
                        if (in.read(lenBuffer) <= 0) break;
                        lenBuffer.flip();
                        int len = (int)lenBuffer.getShort();
//                        Log.d(TAG, "To download: " + Integer.toString(len));
                        buffer.limit(len);
                        for (int read = 0; read < buffer.limit(); read += in.read(buffer));
                        buffer.flip();
                        vpnOutput.write(buffer);

                        buffer.clear();
                        lenBuffer.clear();
                    }
                    buffer.clear();
                    lenBuffer.clear();
                }
            } catch (Exception e) {
                e.printStackTrace();
            }
        }
    }

}
