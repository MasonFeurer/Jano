package nodomain.jano;

import java.net.Socket;
import java.net.InetSocketAddress;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

public class SocketWrapper {
	Socket socket;
	InputStream in;
	OutputStream out;
	
	private static void reportErr(IOException e) {
		nodomain.jano.MainActivity.lastErrCode = "";
		if (e.getCause() instanceof android.system.ErrnoException) {
			android.system.ErrnoException eno = (android.system.ErrnoException)e.getCause();
			String errstr = android.system.OsConstants.errnoName(eno.errno);
			nodomain.jano.MainActivity.lastErrCode = errstr;
		}
		nodomain.jano.MainActivity.lastErr = e.getMessage();
	}
	
	private SocketWrapper() {}
	
	public String getAddress() {
		return this.socket.getLocalAddress().toString();
	}
	public int getPort() {
		return this.socket.getLocalPort();
	}
	
	public static SocketWrapper connect(String addressStr, int port) {
		InetSocketAddress address = new InetSocketAddress(addressStr, port);
		Socket socket = new Socket();
		
		try {
	        socket.connect(address);
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    InputStream in = null;
	    try {
	        in = socket.getInputStream();
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    OutputStream out = null;
	    try {
	        out = socket.getOutputStream();
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    
	    SocketWrapper wrapper = new SocketWrapper();
	    wrapper.socket = socket;
	    wrapper.in = in;
	    wrapper.out = out;
	    return wrapper;
	}
	
	public static SocketWrapper connect(String addressStr, int port, int timeout) {
		InetSocketAddress address = new InetSocketAddress(addressStr, port);
		Socket socket = new Socket();
		
		try {
	        socket.connect(address, timeout);
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    InputStream in = null;
	    try {
	        in = socket.getInputStream();
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    OutputStream out = null;
	    try {
	        out = socket.getOutputStream();
	    } catch (IOException e) {
	        e.printStackTrace();
	        SocketWrapper.reportErr(e);
	        return null;
	    }
	    
	    SocketWrapper wrapper = new SocketWrapper();
	    wrapper.socket = socket;
	    wrapper.in = in;
	    wrapper.out = out;
	    return wrapper;
	}
	
	public int read(byte[] buf) {
		try {
			int readBytes = this.in.read(buf, 0, buf.length);
			if (readBytes == -1) { // End-of-stream occured
				return 0;
			}
			return readBytes;
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
	}
	public int readExact(byte[] buf) {
		int count = buf.length;
		while (count > 0) {
			try {
				int readBytes = this.in.read(buf, buf.length - count, count);
				if(readBytes == -1) { // End-of-stream occured
					return 1;
				}
				count -= readBytes;
			} catch (IOException e) {
				SocketWrapper.reportErr(e);
				e.printStackTrace();
				return -1;
			}
		}
		return 0;
	}
	
	public int write(byte[] buf) {
		try {
			this.out.write(buf);	
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
		return 0;
	}
	public int writeAll(byte[] buf) {
		// java.io.OutputStream.write() will always write every byte in the buffer (if no exception is thrown)
		return this.write(buf);
	}
	
	public int setReadTimeout(int millis) {
		try {
			this.socket.setSoTimeout(millis);
			return 0;
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
	}
	public int readTimeout() {
		try {
			return this.socket.getSoTimeout();
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
	}
	
	public int setNodelay(boolean noDelay) {
		try {
			this.socket.setTcpNoDelay(noDelay);
			return 0;
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
	}
	public int getNodelay() {
		try {
			return this.socket.getTcpNoDelay() ? 1 : 0;
		} catch (IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
	}
	
	public int flush(byte[] buf) {
		try {
			this.out.flush();
		} catch(IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
			return -1;
		}
		return 0;
	}
	
	public void destroy() {
		try {
			this.in.close();
			this.out.close();
			this.socket.close();
		} catch(IOException e) {
			e.printStackTrace();
			SocketWrapper.reportErr(e);
		}
	}
}
