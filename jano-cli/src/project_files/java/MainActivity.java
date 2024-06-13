package nodomain.jano;

// import java.collections.ArrayList;

import androidx.core.view.WindowCompat;
import androidx.core.view.ViewCompat;
import androidx.core.view.DisplayCutoutCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import androidx.core.graphics.Insets;

import com.google.androidgamesdk.GameActivity;

import android.os.Build.VERSION;
import android.os.Build.VERSION_CODES;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.InputMethodManager;
import android.content.Context;
import android.content.ClipboardManager;
import android.content.ClipDescription;
import android.content.ClipData;
import android.content.Intent;

import android.graphics.Bitmap;
import java.nio.ByteBuffer;
// import android.widget.EditText 
// import android.widget.TextView 

public class MainActivity extends GameActivity {
    static {
        System.loadLibrary("main");
    }
    
    public static String lastErr = null;
    public static String lastErrCode = null;
    public static String getLastErr() {
    	return MainActivity.lastErr;
    }
    public static String getLastErrCode() {
    	return MainActivity.lastErrCode;
    }
    
    native public static void onDisplayInsets(int[] cutouts);
	
	native public static void onPictureTaken(byte[] data, int w, int h);
   	
    // ArrayList<EditText> visibleTextFields = new ArrayList();
    
    // public void clearVisibleTextFields() {
    // 	visibleTextFields.clear();
    // }
    // public void addVisibleTextField(String text, int[] viewBounds /* [x, y, w, h] */) {
    // 	EditText field = new EditText();
    // 	field.setText(text, TextView.BufferType.EDITABLE);
    // 	field.setMinimumWidth(viewBounds[2]);
    // 	field.setMinimumHeight(viewBounds[3]);
    // 	field.setX(viewBounds[0]);
    // 	field.setY(viewBounds[1]);
    	
    // 	field.setSelection()
    // 	visibleTextFields.append(field);
    // }
    
    // EditText activeField = null;
    
    // public void setActiveFieldNull() {
    // 	activeField = null;
    // }
    // public void setActiveField(String text, int[] selection, int[] viewBounds, int[] cursorBounds) {
    // 	activeField = new EditText();
    // 	activeField.setSelection(selection[0], selection[1]);
    // 	activeField.setText(text, TextView.BufferType.EDITABLE);
    // 	field.setMinimumWidth(viewBounds[2]);
    // 	field.setMinimumHeight(viewBounds[3]);
    // 	field.setX(viewBounds[0]);
    // 	field.setY(viewBounds[1]);
    // }
    // public bool readActiveField(String textOut, int[] selectionOut) {
    // 	if activeField == null {
    // 		return false;
    // 	}
    // 	textOut = activeField.getText();
    // 	selectionOut 
    // }
    
    public void takePicture() {
    	try {
	    	Intent intent = new Intent("android.media.action.IMAGE_CAPTURE");
			this.startActivityForResult(intent, 123);
		}
		catch (Exception e) {
			e.printStackTrace();
		}
    }
    
    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
    	super.onActivityResult(requestCode, resultCode, data); 
    	if (requestCode == 123 && data != null) {
    		System.out.println("setting recievedImage");
    		Bitmap bitmap = (Bitmap) data.getExtras().get("data");
    		bitmap.setConfig(Bitmap.Config.ARGB_8888);
    		byte[] rawBuf = new byte[bitmap.getByteCount()];
    		ByteBuffer buf = ByteBuffer.wrap(rawBuf);
    		bitmap.copyPixelsToBuffer(buf);
    		onPictureTaken(rawBuf, bitmap.getWidth(), bitmap.getHeight());
    	}
    } 
    
    public static SocketWrapper connectNewSocket(String addressStr, int port, int timeout) {
    	return SocketWrapper.connect(addressStr, port, timeout);
    }
    
    public String getClipboardContent() {
        ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboard != null && clipboard.hasPrimaryClip() && clipboard.getPrimaryClip() != null) {
            CharSequence clip = clipboard.getPrimaryClip().getItemAt(0).coerceToText(MainActivity.this).toString();
            return clip.toString();
        }
        return new String();
    }
    public void setClipboardContent(String text) {
    	if(text == null) {
    		System.err.println("MainActivity.setClipboardContent called with a null String");
    		return;
    	}
    	
    	ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
    	clipboard.setPrimaryClip(new ClipData(new ClipDescription("text", new String[] { "text/html" }), new ClipData.Item(text)));
    }

    public void showSoftKeyboard() {
        View content = findViewById(android.R.id.content);
        if(content == null) {
            System.out.println("findViewById(android.R.id.content) returned null");
            return;
        }
        View view = content.getRootView();
        if(view == null) {
            System.out.println("getRootView() returned null");
            return;
        }
        InputMethodManager imm = (InputMethodManager) view.getContext().getSystemService(
                Context.INPUT_METHOD_SERVICE);
        imm.toggleSoftInput(InputMethodManager.SHOW_FORCED, InputMethodManager.HIDE_IMPLICIT_ONLY);
    }

    public void hideSoftKeyboard() {
        View content = findViewById(android.R.id.content);
        if(content == null) {
            System.out.println("findViewById(android.R.id.content) returned null");
            return;
        }
        View view = content.getRootView();
        if(view == null) {
            System.out.println("getRootView() returned null");
            return;
        }
        InputMethodManager imm = (InputMethodManager) view.getContext().getSystemService(
                Context.INPUT_METHOD_SERVICE);
        imm.hideSoftInputFromWindow(view.getWindowToken(), 0);
    }

    private void createInsetsListener() {
        // Listener for display insets (cutouts) to pass values into native code.
        View content = getWindow().getDecorView().findViewById(android.R.id.content);
        ViewCompat.setOnApplyWindowInsetsListener(content, (v, insets) -> {
            DisplayCutoutCompat dc = insets.getDisplayCutout();
            int cutoutTop = 0;
            int cutoutRight = 0;
            int cutoutBottom = 0;
            int cutoutLeft = 0;
            if (dc != null) {
                cutoutTop = dc.getSafeInsetTop();
                cutoutRight = dc.getSafeInsetRight();
                cutoutBottom = dc.getSafeInsetBottom();
                cutoutLeft = dc.getSafeInsetLeft();
            }
            Insets systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars());

            int[] values = new int[]{0, 0, 0, 0};
            values[0] = Integer.max(cutoutTop, systemBars.top);
            values[1] = Integer.max(cutoutRight, systemBars.right);
            values[2] = Integer.max(cutoutBottom, systemBars.bottom);
            values[3] = Integer.max(cutoutLeft, systemBars.left);
            onDisplayInsets(values);
            return insets;
        });
    }

    private void hideSystemUI() {
        // This will put the game behind any cutouts and waterfalls on devices which have
        // them, so the corresponding insets will be non-zero.
        if (VERSION.SDK_INT >= VERSION_CODES.P) {
            getWindow().getAttributes().layoutInDisplayCutoutMode
                    = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        // From API 30 onwards, this is the recommended way to hide the system UI, rather than
        // using View.setSystemUiVisibility.
        View decorView = getWindow().getDecorView();
        WindowInsetsControllerCompat controller = new WindowInsetsControllerCompat(getWindow(),
                decorView);
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.hide(WindowInsetsCompat.Type.displayCutout());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // When true, the app will fit inside any system UI windows.
        // When false, we render behind any system UI windows.
        WindowCompat.setDecorFitsSystemWindows(getWindow(), true);
        // hideSystemUI();
        createInsetsListener();
        super.onCreate(savedInstanceState);
    }

    protected void onResume() {
        super.onResume();
        // hideSystemUI();

        // View view = this.
        // view.setFocusable(true);
        // view.setFocusableInTouchMode(true);
        // view.requestFocus();
    }
}