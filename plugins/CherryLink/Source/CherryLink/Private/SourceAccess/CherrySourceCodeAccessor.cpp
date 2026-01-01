// Copyright Krishna Teja Mekala. All Rights Reserved.

#include "SourceAccess/CherrySourceCodeAccessor.h"
#include "CherryLinkModule.h"
#include "DesktopPlatformModule.h"
#include "Misc/Paths.h"
#include "HAL/PlatformProcess.h"
#include "Misc/UProjectInfo.h"

#define LOCTEXT_NAMESPACE "CherrySourceCodeAccessor"

void FCherrySourceCodeAccessor::Startup()
{
	RefreshAvailability();
}

void FCherrySourceCodeAccessor::Shutdown()
{
	// Nothing to clean up
}

void FCherrySourceCodeAccessor::RefreshAvailability()
{
	bHasCachedAvailability = false;
	bIsCherryAvailable = IsCherryAvailable();
}

bool FCherrySourceCodeAccessor::CanAccessSourceCode() const
{
	return IsCherryAvailable();
}

FName FCherrySourceCodeAccessor::GetFName() const
{
	return FName("Cherry");
}

FText FCherrySourceCodeAccessor::GetNameText() const
{
	return LOCTEXT("CherryDisplayName", "Cherry");
}

FText FCherrySourceCodeAccessor::GetDescriptionText() const
{
	return LOCTEXT("CherryDisplayDesc", "Open source files in Cherry");
}

bool FCherrySourceCodeAccessor::OpenSolution()
{
	if (!IsCherryAvailable())
	{
		return false;
	}

	// Get the project path
	FString ProjectPath = FPaths::GetProjectFilePath();
	if (ProjectPath.IsEmpty())
	{
		// Try to get from engine
		ProjectPath = FPaths::ProjectDir();
	}

	if (!ProjectPath.IsEmpty())
	{
		return LaunchCherry(FString::Printf(TEXT("\"%s\""), *FPaths::ConvertRelativePathToFull(ProjectPath)));
	}

	return false;
}

bool FCherrySourceCodeAccessor::OpenSolutionAtPath(const FString& InSolutionPath)
{
	if (!IsCherryAvailable())
	{
		return false;
	}

	FString SolutionPath = InSolutionPath;

	// If it's a .sln file, open the directory instead
	if (SolutionPath.EndsWith(TEXT(".sln")))
	{
		SolutionPath = FPaths::GetPath(SolutionPath);
	}

	return LaunchCherry(FString::Printf(TEXT("\"%s\""), *FPaths::ConvertRelativePathToFull(SolutionPath)));
}

bool FCherrySourceCodeAccessor::DoesSolutionExist() const
{
	// Cherry doesn't need a solution file - it works with directories
	return FPaths::DirectoryExists(FPaths::ProjectDir());
}

bool FCherrySourceCodeAccessor::OpenFileAtLine(const FString& FullPath, int32 LineNumber, int32 ColumnNumber)
{
	if (!IsCherryAvailable())
	{
		return false;
	}

	FString Arguments;
	if (LineNumber > 0)
	{
		if (ColumnNumber > 0)
		{
			Arguments = FString::Printf(TEXT("\"%s:%d:%d\""), *FullPath, LineNumber, ColumnNumber);
		}
		else
		{
			Arguments = FString::Printf(TEXT("\"%s:%d\""), *FullPath, LineNumber);
		}
	}
	else
	{
		Arguments = FString::Printf(TEXT("\"%s\""), *FullPath);
	}

	return LaunchCherry(Arguments);
}

bool FCherrySourceCodeAccessor::OpenSourceFiles(const TArray<FString>& AbsoluteSourcePaths)
{
	if (!IsCherryAvailable())
	{
		return false;
	}

	for (const FString& Path : AbsoluteSourcePaths)
	{
		LaunchCherry(FString::Printf(TEXT("\"%s\""), *Path));
	}

	return true;
}

bool FCherrySourceCodeAccessor::AddSourceFiles(const TArray<FString>& AbsoluteSourcePaths, const TArray<FString>& AvailableModules)
{
	// Cherry doesn't need to add source files - it discovers them automatically
	return true;
}

bool FCherrySourceCodeAccessor::SaveAllOpenDocuments() const
{
	// Not implemented - Cherry doesn't expose this capability via command line
	return false;
}

void FCherrySourceCodeAccessor::Tick(const float DeltaTime)
{
	// Nothing to do here
}

bool FCherrySourceCodeAccessor::IsCherryAvailable() const
{
	if (bHasCachedAvailability)
	{
		return bIsCherryAvailable;
	}

	FString ExecutablePath = GetCherryExecutablePath();
	bIsCherryAvailable = !ExecutablePath.IsEmpty() && FPaths::FileExists(ExecutablePath);
	bHasCachedAvailability = true;

	return bIsCherryAvailable;
}

FString FCherrySourceCodeAccessor::GetCherryExecutablePath() const
{
	if (!CachedExecutablePath.IsEmpty())
	{
		return CachedExecutablePath;
	}

	// Look for Cherry in common locations
	TArray<FString> PossiblePaths;

#if PLATFORM_WINDOWS
	// Check in Program Files
	PossiblePaths.Add(TEXT("C:/Program Files/Cherry/cherry.exe"));
	PossiblePaths.Add(TEXT("C:/Program Files (x86)/Cherry/cherry.exe"));

	// Check in AppData/Local
	FString LocalAppData = FPlatformMisc::GetEnvironmentVariable(TEXT("LOCALAPPDATA"));
	if (!LocalAppData.IsEmpty())
	{
		PossiblePaths.Add(FPaths::Combine(LocalAppData, TEXT("Cherry/cherry.exe")));
		PossiblePaths.Add(FPaths::Combine(LocalAppData, TEXT("Programs/Cherry/cherry.exe")));
	}

	// Check in current development location (for development builds)
	PossiblePaths.Add(TEXT("W:/Projects/cherry/target/debug/cherry.exe"));
	PossiblePaths.Add(TEXT("W:/Projects/cherry/target/release/cherry.exe"));
#elif PLATFORM_MAC
	PossiblePaths.Add(TEXT("/Applications/Cherry.app/Contents/MacOS/cherry"));
	PossiblePaths.Add(FPaths::Combine(FPlatformMisc::GetEnvironmentVariable(TEXT("HOME")), TEXT("Applications/Cherry.app/Contents/MacOS/cherry")));
#elif PLATFORM_LINUX
	PossiblePaths.Add(TEXT("/usr/local/bin/cherry"));
	PossiblePaths.Add(TEXT("/usr/bin/cherry"));
	PossiblePaths.Add(FPaths::Combine(FPlatformMisc::GetEnvironmentVariable(TEXT("HOME")), TEXT(".local/bin/cherry")));
#endif

	// Try to find Cherry
	for (const FString& Path : PossiblePaths)
	{
		if (FPaths::FileExists(Path))
		{
			CachedExecutablePath = Path;
			return CachedExecutablePath;
		}
	}

	// Try to find in PATH
	FString PathEnv = FPlatformMisc::GetEnvironmentVariable(TEXT("PATH"));
	TArray<FString> PathDirs;
	PathEnv.ParseIntoArray(PathDirs, FPlatformMisc::GetPathVarDelimiter());

	for (const FString& Dir : PathDirs)
	{
#if PLATFORM_WINDOWS
		FString ExePath = FPaths::Combine(Dir, TEXT("cherry.exe"));
#else
		FString ExePath = FPaths::Combine(Dir, TEXT("cherry"));
#endif
		if (FPaths::FileExists(ExePath))
		{
			CachedExecutablePath = ExePath;
			return CachedExecutablePath;
		}
	}

	return FString();
}

bool FCherrySourceCodeAccessor::LaunchCherry(const FString& Arguments) const
{
	FString ExecutablePath = GetCherryExecutablePath();
	if (ExecutablePath.IsEmpty())
	{
		UE_LOG(LogCherryLink, Warning, TEXT("Cherry executable not found"));
		return false;
	}

	UE_LOG(LogCherryLink, Log, TEXT("Launching Cherry: %s %s"), *ExecutablePath, *Arguments);

	FProcHandle Handle = FPlatformProcess::CreateProc(
		*ExecutablePath,
		*Arguments,
		true,  // bLaunchDetached
		false, // bLaunchHidden
		false, // bLaunchReallyHidden
		nullptr,
		0,     // PriorityModifier
		nullptr,
		nullptr
	);

	if (Handle.IsValid())
	{
		FPlatformProcess::CloseProc(Handle);
		return true;
	}

	UE_LOG(LogCherryLink, Warning, TEXT("Failed to launch Cherry"));
	return false;
}

#undef LOCTEXT_NAMESPACE
