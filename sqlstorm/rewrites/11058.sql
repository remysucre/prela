
SELECT 
    pt.Name AS PostType,
    COUNT(p.Id) AS TotalPosts,
    SUM(COALESCE(date_diff('microsecond', p.CreationDate, p.LastActivityDate), 0))::DOUBLE / COUNT(*) / 1e6 AS AvgPostAgeInSeconds,
    SUM(p.ViewCount) AS TotalViews,
    AVG(p.Score) AS AvgScore,
    SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
    SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
    SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalPostsClosed
FROM 
    Posts p
INNER JOIN 
    PostTypes pt ON p.PostTypeId = pt.Id
LEFT JOIN 
    Votes v ON p.Id = v.PostId
LEFT JOIN 
    PostHistory ph ON p.Id = ph.PostId
GROUP BY 
    pt.Name
ORDER BY 
    TotalPosts DESC;
