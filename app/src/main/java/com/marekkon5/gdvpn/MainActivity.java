package com.marekkon5.gdvpn;

import androidx.appcompat.app.AppCompatActivity;

import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.util.Log;
import android.view.View;
import android.widget.Button;
import android.widget.Toast;

import com.google.android.gms.auth.api.signin.GoogleSignIn;
import com.google.android.gms.auth.api.signin.GoogleSignInClient;
import com.google.android.gms.auth.api.signin.GoogleSignInOptions;
import com.google.android.gms.common.api.Scope;
import com.google.api.client.extensions.android.http.AndroidHttp;
import com.google.api.client.googleapis.extensions.android.gms.auth.GoogleAccountCredential;
import com.google.api.client.json.gson.GsonFactory;
import com.google.api.services.drive.Drive;
import com.google.api.services.drive.DriveScopes;

import java.util.Collections;

public class MainActivity extends AppCompatActivity {

    private static final int VPN_REQUEST_CODE = 0x0F;
    private static final int REQUEST_CODE_SIGN_IN = 0xA;

    Context context;
    GoogleAccountCredential googleAccountCredential;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        context = this;

        Button button = findViewById(R.id.button);
        button.setOnClickListener((View view) -> {

            // Check for gdrive
            if (googleAccountCredential == null) {
                Toast.makeText(getApplicationContext(), "Google drive not authorized yet", Toast.LENGTH_SHORT).show();
                return;
            }

            // Start VPN
            GDVPN.GOOGLE_ACCOUNT_CREDENTIAL = googleAccountCredential;
            Intent intent = GDVPN.prepare(context);
            if (intent != null) {
                startActivityForResult(intent, VPN_REQUEST_CODE);
            } else {
                onActivityResult(VPN_REQUEST_CODE, RESULT_OK, null);
            }

        });

        Button button1 = findViewById(R.id.button2);
        button1.setOnClickListener(view -> {
            // Google drive sign in
            GoogleSignInOptions signInOptions = new GoogleSignInOptions.Builder(GoogleSignInOptions.DEFAULT_SIGN_IN)
                    .requestEmail()
                    .requestScopes(new Scope(DriveScopes.DRIVE_FILE))
                    .build();
            GoogleSignInClient client = GoogleSignIn.getClient(context, signInOptions);
            startActivityForResult(client.getSignInIntent(), REQUEST_CODE_SIGN_IN);
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == VPN_REQUEST_CODE && resultCode == RESULT_OK) {
            startService(new Intent(this, GDVPN.class));
        }

        if (requestCode == REQUEST_CODE_SIGN_IN) {
            // Failed
            if (resultCode != RESULT_OK) {
                Toast.makeText(getApplicationContext(), "Failed logging in!", Toast.LENGTH_SHORT).show();
                return;
            }

            // Get instance from auth
            GoogleSignIn.getSignedInAccountFromIntent(data).addOnSuccessListener(googleSignInAccount -> {
                GoogleAccountCredential credential = GoogleAccountCredential.usingOAuth2(this, Collections.singleton(DriveScopes.DRIVE_FILE));
                credential.setSelectedAccount(googleSignInAccount.getAccount());
                googleAccountCredential = credential;
            });
        }
    }
}