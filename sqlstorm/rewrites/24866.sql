WITH UserVoteStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
        COUNT(DISTINCT P.Id) AS TotalPosts,
        COUNT(DISTINCT CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN P.Id END) AS AnsweredQuestions,
        DENSE_RANK() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS VoteRank
    FROM Users U
    LEFT JOIN Posts P ON U.Id = P.OwnerUserId
    LEFT JOIN Votes V ON P.Id = V.PostId
    GROUP BY U.Id, U.DisplayName
),
ClosedPostCount AS (
    SELECT 
        PH.UserId,
        COUNT(PH.Id) AS ClosedPosts
    FROM PostHistory PH
    WHERE PH.PostHistoryTypeId = 10
    GROUP BY PH.UserId
),
TagStats AS (
    SELECT 
        T.TagName,
        COUNT(DISTINCT P.Id) AS PostCount,
        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
    FROM Tags T
    JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
    LEFT JOIN Comments C ON P.Id = C.PostId
    GROUP BY T.TagName
    HAVING COUNT(DISTINCT P.Id) > 5
),
FinalResults AS (
    SELECT 
        UVS.UserId,
        UVS.DisplayName,
        UVS.UpVotesCount,
        UVS.DownVotesCount,
        COALESCE(CPC.ClosedPosts, 0) AS ClosedPosts,
        UVS.TotalPosts,
        UVS.AnsweredQuestions,
        UVS.VoteRank,
        TS.TagName,
        TS.PostCount,
        TS.CommentCount
    FROM UserVoteStats UVS
    LEFT JOIN ClosedPostCount CPC ON UVS.UserId = CPC.UserId
    LEFT JOIN TagStats TS ON UVS.VoteRank = 1 
    ORDER BY UVS.VoteRank, TS.PostCount DESC
)
SELECT 
    UserId,
    DisplayName,
    UpVotesCount,
    DownVotesCount,
    ClosedPosts,
    TotalPosts,
    AnsweredQuestions,
    TagName,
    PostCount,
    CommentCount,
    CASE 
        WHEN ClosedPosts > TotalPosts / 2 THEN 'Major Contributor to Closed Posts'
        WHEN UpVotesCount >= DownVotesCount THEN 'Positive Impact'
        ELSE 'Neutral Contribution'
    END AS ContributorImpact
FROM FinalResults
WHERE VoteRank <= 10
ORDER BY ContributorImpact, UpVotesCount DESC, DownVotesCount ASC, UserId, TagName
LIMIT 100;