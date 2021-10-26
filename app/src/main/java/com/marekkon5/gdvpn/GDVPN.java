package com.marekkon5.gdvpn;

import android.app.PendingIntent;
import android.content.Intent;
import android.net.VpnService;
import android.os.ParcelFileDescriptor;

import com.google.api.client.extensions.android.http.AndroidHttp;
import com.google.api.client.googleapis.extensions.android.gms.auth.GoogleAccountCredential;
import com.google.api.client.json.gson.GsonFactory;
import com.google.api.services.drive.Drive;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class GDVPN extends VpnService {

    private static final String TAG = "GDVPN";
    public static GoogleAccountCredential GOOGLE_ACCOUNT_CREDENTIAL = null;

    private ParcelFileDescriptor vpnInterface;
    private PendingIntent pendingIntent;
    private ExecutorService executor;
    List<String> fileIds = new ArrayList<String>();

    @Override
    public void onCreate() {
        super.onCreate();
        executor = Executors.newFixedThreadPool(4);

        // Create VPN interface
        try {

            if (vpnInterface != null) return;
            Builder builder = new Builder();
            builder.addAddress("10.0.0.2", 32);
            builder.addRoute("0.0.0.0", 0);
            builder.addDnsServer("1.1.1.1");
            builder.addDisallowedApplication(getApplicationContext().getPackageName());
            builder.addDisallowedApplication("com.marekkon5.gdvpn");
            builder.addDisallowedApplication("com.google.android.gms");
            vpnInterface = builder.setSession(getString(R.string.app_name))
                    .setConfigureIntent(pendingIntent)
                    .establish();

            Thread thread = new Thread(() -> {
                try {
                    // Create google drive
                    Drive googleDriveService = new Drive.Builder(
                            AndroidHttp.newCompatibleTransport(),
                            new GsonFactory(),
                            GOOGLE_ACCOUNT_CREDENTIAL
                    ).setApplicationName("GDVPN").build();
                    // Open connection
                    VPNConnection connection = new VPNConnection(vpnInterface.getFileDescriptor(), googleDriveService);
                    connection.connect("10.10.10.14", 42069);
                } catch (Exception e) {
                    e.printStackTrace();
                }
            });
            thread.start();

        } catch (Exception e) {
            e.printStackTrace();
        }

    }

    @Override
    public void onDestroy() {
        super.onDestroy();
        try {
            executor.shutdown();
            vpnInterface.close();
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        return START_STICKY;
    }

}
