WITH UserPostMetrics AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COALESCE(SUM(vt.VoteCount), 0) AS TotalVotes,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        COUNT(DISTINCT c.Id) AS TotalComments,
        DENSE_RANK() OVER (ORDER BY COALESCE(SUM(vt.VoteCount), 0) DESC) AS VoteRank
    FROM Users AS u
    LEFT JOIN Posts AS p ON u.Id = p.OwnerUserId
    LEFT JOIN (
        SELECT 
            PostId,
            COUNT(*) AS VoteCount
        FROM Votes
        GROUP BY PostId
    ) AS vt ON p.Id = vt.PostId
    LEFT JOIN Comments AS c ON p.Id = c.PostId
    GROUP BY u.Id, u.DisplayName
),
PostEngagement AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) / 3600 AS AgeInHours,
        COUNT(c.Id) AS CommentCount,
        COUNT(DISTINCT v.UserId) AS UniqueVoteCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM Posts AS p
    LEFT JOIN Comments AS c ON p.Id = c.PostId
    LEFT JOIN Votes AS v ON p.Id = v.PostId
    GROUP BY p.Id, p.Title, p.CreationDate
),
ClosePostAnalysis AS (
    SELECT 
        ph.PostId,
        CASE 
            WHEN COUNT(*) > 0 THEN 'Closed'
            ELSE 'Active'
        END AS PostStatus,
        STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReasonTypes
    FROM PostHistory AS ph
    JOIN CloseReasonTypes AS ctr ON ph.Comment IS NOT NULL AND ph.PostHistoryTypeId = 10
    GROUP BY ph.PostId
),
FinalMetrics AS (
    SELECT 
        upm.UserId,
        upm.DisplayName,
        pm.PostId,
        pm.Title,
        pm.AgeInHours,
        pm.CommentCount,
        pm.UniqueVoteCount,
        pm.UpVotes,
        pm.DownVotes,
        cpa.PostStatus,
        COALESCE(cpa.CloseReasonTypes, 'N/A') AS CloseReasonTypes
    FROM UserPostMetrics AS upm
    JOIN PostEngagement AS pm ON upm.TotalPosts > 0
    LEFT JOIN ClosePostAnalysis AS cpa ON pm.PostId = cpa.PostId
    WHERE upm.VoteRank <= 10
)
SELECT 
    UserId,
    DisplayName,
    PostId,
    Title,
    AgeInHours,
    CommentCount,
    UniqueVoteCount,
    UpVotes,
    DownVotes,
    PostStatus,
    CloseReasonTypes
FROM FinalMetrics
ORDER BY UpVotes DESC, CommentCount DESC, PostId, UserId
LIMIT 100;