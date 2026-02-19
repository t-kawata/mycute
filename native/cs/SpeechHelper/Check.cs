using System;
using Windows.Media.SpeechRecognition;

public class Check
{
    public static void Main()
    {
        var r = new SpeechRecognizer();
        // This line should fail if the property does not exist
        var session = r.ContinuousRecognitionSession;
        Console.WriteLine("Session property exists.");
    }
}
